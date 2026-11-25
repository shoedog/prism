# Lane 9: finish JavaScript parameter binding — design

Stage D design of record, pending both design reviews. Stage M passed every slice gate. Section 5 gives the implementable rules and section 9 identifies the losses and unresolved correctness checks. Tracking issues: #595 (rest and bare-arrow formals), #434 (calls through a callback
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
| The unparenthesised arrow parameter was left unbound on purpose (decision D12): with it bound, a measurement on the TypeScript sources gave 57 new edges of which 26 were graded Exact and 1 was correct. The cause was calls resolving Exact to functions whose name is only inferred from where they are assigned. A later repair refused such targets edge by edge. A minimal reproduction of that cause no longer reproduces on main; the census had yet to repeat that measurement at this historical revision. Stage M's completed repeat is in sections 5–6. | `docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prA.md` (D12, probe P11); probe |
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
(chosen after M4): `forEach`, `map`, `filter`, `find`, `findIndex`, `some`, `every`,
`flatMap` (receiver elements into the callback's first parameter), `reduce` and `reduceRight` (into the
second, also the initial accumulator when the initializer is omitted), `then` (the fulfilled value into the first). The edge runs from the receiver's node (section 5,
slice D) to the callback parameter's definition. A user-defined method of the same name gets the same edge;
that is why the label is not `exact`. The oracle credits an edge only when the checker resolves the call to
the method declared in the TypeScript library files and establishes delivery in the applicable element or seed role. A rest Def may collect that role.

**C. Treat the callback as called by the enclosing function** (a synthetic call edge with no data). It
helps navigation but moves no data. Not chosen for this lane.

For calls through a parameter, M5 supports #434's fallback from visible Exact Source callers, including anonymous callable identities. Section 5 settles runtime slots, writes, capped forwarding and storage. The D12 repeat supports ordinary bare formals with the measured incoming refusal in C and the existing outgoing check.

## 5. Architecture and row decisions

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
| A. Spread arguments | the binding step computes runtime offsets (rule above): arguments after a known-length array-literal spread keep their correct positions; after any other spread positional binding stops; the spread itself binds nothing (A uses literal length only and emits no fixed edge from the literal's elements). The spread is detected from the argument's syntax; the call site's spread flag is not set for JavaScript, because arity-based resolution reads it. The JS-family argument lookup uses the call identity rule | removes proved-wrong and unproved edges and the chain-misbound ones; moves a misplaced known-length edge to its correct formal. It removes no edge whose offset is exact and correct; an unproved removal can be correct at run time, and the oracle reports every removed edge as proved-wrong, unproved or correct (a correct removal is a STOP). Removing an edge can change the per-edge refusal that guards new parameter sources (section 7, refusal accounting); each such change is listed |
| B. Rest parameters | An eligible ordinary Use, wholly trailing spread operand or dense literal element reaches the identifier rest Def at a proved lower bound | Inherit the call label |
| C. Bare parameters | Bind the byte-identified formal under admission and the ordinary cross-file NameOnly refusal | Inherit the call label |
| D. Built-in higher-order callbacks | Bind a represented receiver to an eligible callback Def for a supported role | `nameonly/builtin_callback` |
| E. Calls through a parameter | Derive a bounded set from admitted Exact Source callers at the holder's stable lexical slot | `argument_follow` targets, `nameonly/call_nameonly` payload edges |

### Reading the matrix

The row ledger is part of this section. It assigns a rule and required disposition to every ID, including negative assertions, mixed-slice rows and controls. An unchanged row requires all its original assertions, with their source coordinates, label and multiplicity. A changed row has a replacement or a named loss. A RED receipt describes the supplied fixture on its pinned base. It does not decide what JavaScript does. [Row ledger and M3 grading keys](RULES-INPUT-NOTES.md)

The existing lane 3 literal-leaf producer (`src/cpg/object_literal_args.rs`, `argument_leaves` and `exact_singleton`) separately refuses any earlier spread and any non-Exact-singleton call. An object literal after a known spread has a definite slot, but its field Use → callee read is subject to that producer. In s1-1/s5-3 record the missing leaf and keep all wrong-slot prohibitions. M1 found no resolved trailing-spread site on the three corpora. An owner decision would be needed to change that shared contract. The same producer records 44 wrong NameOnly-singleton sites in its own earlier census. That is the historical cost evidence for keeping its Exact-singleton gate in s5-28. M5 counts 3,782 proposed Use → Def rows and supplies no marginal literal-leaf count.

### B. Rest elements

Find the byte-verified identifier rest Def of the **runtime implementation**. Its index `r` counts ordinary parameters, including a default, destructured fixed parameter or TS parameter property. A TS `this` and overload declarations consume no runtime slot. Keep entry-Def eligibility, including the existing captured-rebinding exclusion. A write to a shadow or to `rest[i]` leaves the binding intact. A rest edge means a value can be an **element** of the rest array. It establishes neither array identity nor a dependence of `length` on that value (b01a–d, b04, b05a/b, s1-7, s2-4/5, s3-5/6, s4-3/5, x1-6, x2-3, x3-4).

* Bind an ordinary source Use if its **least** runtime offset is at least `r`. Exact and least offsets diverge after an unknown-length spread. Below `r`, an unknown offset supplies neither a fixed nor a rest destination. In b03, `xs` and `v` could fill the fixed prefix. In b09a only `w` has reached rest for every length. Later fixed arguments and known spreads continue to raise the floor. In x2-5 a spread containing a spread still contributes a floor of zero under A. b02a, b09b/c, t3-2, s1-6 and s5-5 exercise the other boundaries.
* A dense literal spread determines subsequent positions in A. **A emits no fixed-slot edge from its elements** (t3-1). B can bind an existing element Use at literal index `k` to rest when `least[start] + k ≥ r`. Inspect actual top-level element expressions using the ordinary value/path extractor. Do not flatten an array-valued element or mine an object field for an element. In s5-7 bind `value → tail`. In x1-5 bind `extra` and `last` to `tail` and record the missing fixed `first → head`. For an opaque spread with an operand Use, bind that operand's path/base by the ordinary extractor only when the **start** is already in rest. A straddling opaque operand has no proved destination (b02b/c, b07, s2-6, s3-4, s4-4). Unsupported operands still contribute their A offset.
* Each distinct source occurrence keeps the call label, including NameOnly (b06). No rest leaf expansion, implicit-constructor or bound-function adapter is created. b08 keeps the existing outgoing refusal. M2 measured **482** admitted sites, **53** supplied only by a spread operand, **1** refused cross-file site and **991** sites without an eligible Use. Zero admitted sites had a spread before the rest index or before the eligible argument. These last zeros describe the admitted population. M1 found zero known-length literals among ten captured trailing-spread sites. Neither census counted all terminal literal leaves. M6 found zero first breaks at a rest Def.

### C. Ordinary bare-arrow binding

A bare slot uses the same occurrence, runtime position, Def and label as a parenthesised identifier slot. Resolve the function node before matching its formal bytes. This includes a uniquely identified anonymous/IIFE/default-export owner as well as a named declaration. Keep the ordinary lexical and existing inferred-name protections, value-space import rules and per-edge outgoing seed/reachability check. A spelling alone, a type-only import or an enclosing expression containing an arrow supplies no identity. Apply admission to both Use → Def and the existing Exact-singleton object-literal leaf → read producer. A bare Def remains visible even when its incoming edge is refused. Rows k01a/b, k02–k07b, s1-9/10/11, s3-7/8/9, s4-6/7/8, s5-9/10/11/12, x2-6/7 and x3-6 distinguish these mechanisms. Lists with a slash abbreviate IDs from the same enumerator round.

One further incoming refusal applies to **ordinary C bindings only**. Withhold a newly enabled bare-target edge when its call label is NameOnly and the caller and target files differ. Compare the target on each candidate edge. Keep same-file NameOnly and admitted cross-file Exact. A parenthesised slot, a B rest edge and the independently grounded D/E routes do not enter this predicate. It needs neither checker results nor a corpus pathname at runtime. On the M3 probe it costs X/T/SB **0/48/2 rows**, comprising **6 WRONG, 44 UNDECIDED, 0 CORRECT**. All six WRONG are three `write` calls in `compiler/emitter.ts` (4111, 4273, 4982) paired with two foreign `write` declarations. Without the refusal the probe has 6 WRONG among 53 NameOnly, over the 5% limit. A blanket NameOnly refusal would also discard two CORRECT same-file SB rows.

Of 195 added probe rows, the final admission removes another 14 T Exact `free_single` rows. The disjoint predicates leave **131 rows, 116 CORRECT and 15 UNDECIDED**, on the supplied probe ledger. Fourteen surviving Exact rows (5 T, 9 SB) and one T NameOnly row earn zero credit. Missing runtime-callee or argument-value witnesses do not justify another product veto. The three historical D12 examples added zero rows. Recheck the two deltas and the outgoing paths on the actual head, including k08a/b's return-flow counters. Section 7's rate and zero-WRONG-Exact gates still apply.

### D. Built-in higher-order callbacks

**Receiver.** Start at the resolved site's own call-expression bytes. Require an ordinary or optional member invocation with an attached receiver and a static public table name. Accept dotted names, escaped identifiers, static quoted bracket names, grouping and erased TS wrappers when they preserve the reference. Reject a dynamic key, private name, detached comma call, constructor, tag or an unlisted method. Permit identifiers, static member paths and `this` paths through optional segments. Prefer the nonempty Use of the *complete receiver path* at its own bytes, then the projection Use of `receiver.method` exactly spanning that callee, then the zero-width base Use at the receiver start in that owner. Use only the first matching representation. No match means no edge. Never borrow an inner call's receiver, an argument to a producer, a condition, a computed index or a same-line neighbour. In h09a/b/c the receiver is a **call result**. Require each callback Def to exist, then enable each negative into `k`, `y`, `r` respectively, including negatives from `o`, `arr/f` and `u`. h07a–g, s2-9/10/11, s4-10/11, s5-17, x1-10/13, x2-8 and x4-7 pin the extraction.

**Callback.** Read the value at runtime argument zero, including after a proved empty spread or inside a dense literal at a known position. Inline arrow, async arrow, function expression (including named or generator) and transparent wrappers identify the exact callable node in place. A bare identifier must lexically denote one live same-file function declaration or immutable `const` directly initialised with a callable, at the occurrence. Hoisting, TDZ, shadowing and writes matter. Use the callable's own bytes and Def. An import or alias of an import, formal, mutable binding, member, bound function, callback factory or another argument does not prove this identity. A bare arrow binds here even if ordinary C is disabled. h04, h05a–c, t2-5, t4-5, t5-2/3, s1-12/13, s2-12, s3-10/13, s4-12, s5-15/16/18, x1-14 and x5-8 separate identity from method-name evidence.

**Role.** `forEach/map/filter/find/findIndex/some/every/flatMap` feed slot 0. `then` feeds only the fulfillment value at slot 0 of argument 0. `reduce/reduceRight` feed slot 1. With a proved **omitted initializer** they can also feed slot 0 of the first callback from the first/last visited element. An explicit `undefined` is an initializer. This extra seed edge does not summarize later accumulator values. Bind a read identifier Def at that runtime slot, or the read identifier rest Def covering it. Deduplicate the same source/Def if two supported roles enter the same rest. Skip TS `this`. A destructured parameter occupies its slot without exposing an identifier Def. The index, Map key, second copy of a Set value, collection, `thisArg`, explicit initializer and rejection reason have no D edge. The library role, actual dispatch and evidence of delivery determine checker credit. h01a–h03b, h06a–c, t1-11/12, t2-6, t3-3/4, s1-14/17, s2-13/14/15, s3-12, s4-9/13, x1-11/12 and x3-9 pin the roles. Keep one proposed transfer per source occurrence, destination and label.

**Label and descent.** A D edge always carries `NameOnly(BuiltinCallback)`, wire spelling `nameonly/builtin_callback`. Choose badness key **(6, 0)** in `FlowConfidence::worst`, after `CallNameOnly` (5), `AliasUnstable` (4), `CfgIncomplete` (3), `SameLine` (2), `Killed` (1) and `Exact` (0). A tie for two distinct doubts would make operand order select the spelling. Test both orders, associativity, idempotence and the existing killed-line tie break. Review reachability may use D/E edges. Under the Exact trace ceiling their **own** labels terminate descent as `BoundaryExited/not_exact`, including when the same callback has a direct Exact call on the same line. Lane 8's recorded answer is **yes**, in its [DESIGN §5.2](../lane8-taint-parity/DESIGN.md) and [PLAN I-guard](../lane8-taint-parity/PLAN.md). I-guard follows I-1 and uses containment for byte-carrying occurrences. Match the doubts: lane 9 h13 ↔ lane 8 n02 (builtin), e14 ↔ n01 (call). The parenthetical h13/e14 examples in lane 8's row table are transposed. D and E2 each wait for I-guard to land and their respective integrated boundary test to pass. That commitment supplies no merged-test receipt. h11, h13, e10, e14, s3-14, x2-9 and x3-8 cover the two consumers.

M4 admitted **3,910** sites, **1,336/1,356** typed library declarations, 20 repository and 2,554 untyped. The role sample graded 200 CORRECT, 0 WRONG, 400 UNDECIDED out of 600. Its 200-per-corpus sample preceded the reducer-seed extension. The receiver test cost **10,114** missing nodes, with zero call-result, chain, literal or separately classified `this_path` conversions. Existing path and projection nodes can still qualify a particular `this` path. Callback admission cost **1,707** sites counted as `callback_not_admitted`. There were **1,142** without an observed read Def, **409** candidate sites with bare arrows and **88** with same-file named callbacks. Rejoin those 88 keys on the implementation and count any rejected live-binding proofs. M4 did not classify reassigned declarations within this group. Skipped roles have **142** whole-array reads and **115** `thisArg` reads. M4 saw **742** reducer-name callback sites before filtering. It did not split omitted seeds, rest capture, sparse arrays, own overrides or lazy consumption. The ledger bounds those examples under these measured rules. They introduce no independent suppression of eligible name-only candidates.

### E. Calls through a parameter

**Source and payload.** Apply #434 §3 rules 1, 3, 5 and 7 only to a JS/TS/TSX Source call or JSX component occurrence whose callee resolves lexically to a holder parameter (including a proved body destructure). Use the binding declaration at the callee occurrence, the runtime implementation and its full holder identity. Transparent optional and TS syntax retain that occurrence. A receiver/member invocation, scalar local alias, tag, constructor through a parameter or synthetic invocation does not become a new E site. An ordinary `new Runner(handler, x)` can itself be a Source caller of an Exact holder. Existing resolutions survive. For an empty `UnknownName` or `LocalBindingUnproven` outcome, run E **after anonymous-caller admission and before family filtering**, retaining existing multi-occurrence checks. Use only source callers with exactly one Exact target equal to the holder, lexical admission at **every** forwarding hop, and an actual callable witness. Ignore derived/synthetic reverse edges, including D. An exported or escaping holder keeps its witnessed subset. The target kind is `argument_follow`, always NameOnly, with `via_alias_route` false. Empty leaves the old drop reason. Payload uses ordinary fixed slots and B's rest rule per target, all `nameonly/call_nameonly`. The independently identified bare-arrow Def binds even with C disabled (e02a). A missing payload Use/Def loses only the data edge. The target may still resolve. Lane 3's existing Exact-singleton literal-leaf contract bounds s5-28. Derived calls provide no Exact return certification. See e01–e16b, s1-25/28/29, s2-28/29, s3-15/22/23, s4-16/21/23, s5-28 and x3-11/17.

**Stable holder.** #434 rule 2 discards entry evidence for a binding written **anywhere** in its holder, including a captured write in a nested callable or a parameter initializer. Include direct, compound, logical, update, destructuring, `for in/of`, initialised re-declarations and hoisted replacements of the same binding. A later or conditionally dead write still costs an earlier real call (t1-9, s4-17). Resolve the declaration in the parameter environment, body, nested block, catch or closure. A shadow write, `var cb` without initializer, a copied default or a property write on a function object does not rebind that entry identity. Direct intrinsic eval and dynamic `with` lookup defeat lexical stability when they can affect the binding. Shadowed or indirect eval does not. For sloppy simple ordinary functions track live mapped `arguments` indices, only supplied positions and the last duplicate. Assignment, update, live descriptor writes, potentially mutating reflective operations or escape of that actual object can invalidate the matching formal. A prior deletion detaches an index. Strict, module, arrow and non-simple parameter lists have no own such map. An arrow may capture its enclosing `arguments`. Distinguish a spread copy, another function's arguments, a shadow, or a proved different write receiver. Never recover the original target from an unknown mutation. Inputs include e06a, t1-9/10, t2-7/8/9/11, t3-7/8, t4-6, s2-16/17/19/21/27, s3-16/17, s4-17/18/19/24, s5-19/25/26, x1-23, x2-15/16, x4-13/16 and x5-10/12.

**Value and slot path.** Keep `(holder identity, zero-based runtime position, path)`. The path is an ordered sequence of typed object-key and array-position steps, empty for a plain formal. Preserve the local binding occurrence/default program when two declarations select the same path. Use the source key in a rename, all nested keys, literal numeric property keys canonicalised to JS strings, and array indices counting elisions. Never silently truncate to one step, use the local alias as a key or concatenate unescaped keys with dots. TS `this` has no slot. A uniquely bound `const {key: cb} = props` can use the parameter's path if the read and source value remain proved. Slot evidence stays paired with each individual caller.

For a parameter destructure, read the property at entry, after all caller arguments have been evaluated. For an in-body destructure, read at its own occurrence. A write or deletion of the selected property or an ancestor before that read can make the entry literal stale. A computed or aliased write with an unknown affected key cannot prove the entry value survives. A proved disjoint write preserves it. Once the callable has been copied into an eligible binding, changing the container does not rebind the copy. A later write to the copied binding itself still invokes the holder-wide write rule. s4-20 supplies the unmodified path. x3-14 separates argument evaluation from entry, x4-10 shows why the two read occurrences matter and x4-13 separates a function property from its binding. M5 counted 8/152/6 body-destructure sites on X/T/SB.

Read a value at an exact runtime position by A's rules. An unknown spread at/before a desired slot does not prove a value or its absence. Known prefixes, subsequent known positions, transparent syntax and literal element identities are usable. Evaluate ordinary object/array **literal** selections in source order. For JSX use the explicit winning named attribute in argument zero. A spread alone cannot establish that attribute, and a child is not an explicit attribute. A later known own entry wins. A later unresolved spread/computed write hides a possibly overwritten key. A later explicit key can re-establish it. A custom prototype, getter, unknown iterator, symbol by description, or nonliteral object cannot be used to prove a missing own data value. A final setter-only property reads as undefined. Follow only proven terminal values from local declarations, stable const aliases, imports/namespaces, an inline callable's own span (including defaults and concise methods), and the established `useCallback` wrapper. For `?:`, `||`, `??` take possible returned branches and for `&&` only the right operand can supply a callable when the left is truthy. Carry the missing/undefined alternative through unions independently. Missing actual/known missing key or proven undefined (including unshadowed `undefined` and `void`) selects the declaration's default. `null`, false, a shadow with a value, a hole in evidence or an absent caller does not. Evaluate outer defaults before element defaults in declaration order against the same caller. An earlier entry binding may supply a default, a later uninitialised one cannot. A JSX boolean attribute is supplied. In-body accessors, mutable heap state, arbitrary call results, bound receivers, `await/yield`, decorator execution, JSX children/factory return values and general control-flow interpretation furnish no new terminal. An admitted JSX Source caller is a syntactic props witness. A custom lowering may invalidate that projection (x4-18). M5 has no cost cell for suppressing all such callers, so no factory-specific veto is authorised. Keep independently witnessed targets. The E rows in the ledger enumerate every case, including e09b, e15, s1-18/19/20/22/27/30, s2-18/20/24/25/26, s3-18/19/20/21, s4-14/15/20 and x4-10/12/15/18.

**Forwarding and cap.** Count holder-caller lookups per path. Direct ground is depth 1, a relay's ground is depth 2, a third lookup contributes nothing. Start empty. Forward only an admitted caller's formal and keep the slot/path on the recursion stack. A cycle contributes no target of its own. Do not mark a slot globally visited, cache a depth-truncated empty as a complete answer, or bootstrap from an ArgumentFollow resolution. A grounded sibling path or a different grounded slot of the same recursive holder remains available. Union admissible **callable identities**, verify their declaration/anonymous owner spans, deduplicate, then cap per invoked slot. Count aliases, repeated callers, and duplicate routes to the same callable once. Distinct files and separate anonymous functions on one line remain distinct. Unrepresentable same-ID declarations do not collapse into a proved target. At 1–8 keep the complete set, at 9+ drop the complete set as `ArgFanout`, counted per affected eligible site. Another slot on the line keeps its own set. No branch-sensitive target narrowing or adapter offsets are implied by deduplication. e05, e08a/b, e13, t4-10, s1-23/24/26, x1-16/21/24, x2-17/22, x4-19 and x5-13/15/18 distinguish these cases.

**Index ownership.** E0 stays move-only. In E1 put the serializable `ArgIndex` type, compute, stats and lookup in `src/resolution/js_ts/arg_follow.rs` and its value module. Store one **whole-graph** field on the existing crate-private `CallGraph.js_anonymous` storage object, `resolution::js_ts::anonymous::ProofIndex` in `src/resolution/js_ts/anonymous.rs`. Give that object separate `files` and `arg_follow` fields. The latter grants no anonymous-admission authority. The existing field and initializers in `src/call_graph.rs` require zero added lines. Build the per-file proofs with an empty index. Invalidate the whole derived value on removals and merges. After all files and proofs are assembled, compute with E disabled/empty and replace the index once in `src/call_graph/indirect.rs`. Never union partial outcomes. Read it by site file, full span and effective callee, then slot and declaration if needed. Persist it in the versioned graph cache, restore it on a full hit and rebuild on a partial hit, including caller deletion. No new CallSite fields, process-global map or per-resolution recursion. Check serial, parallel, cold, full and partial paths, including 8→9→8 and anonymous-proof-only equality separately from whole-graph equality. E1 changes facts and counters only.

M5 measured **1,253** sites with a payload and 1–8 targets, including **368** in anonymous callers and **176** with a bare-arrow target Def. It recorded **135** fanout drops (**105** with payload), **284** foreign holder callers affecting **12** parameter sites, and **1,460** reassigned parameter sites. Its open-reason counts include **265** cycle and **2** depth cases, with overlap. Existing resolved parameter sites **0/149/356** keep their answers. The census found **19,622** eligible-site dispositions without an admitted target, **89** without a target Def and **773** without a payload Use *after* target selection. These are separate reasons for missing a conversion. E1 must replay M5's candidate keys with the refined write/path proofs and report newly withheld caller contributions and sites, including which still have an independent ground. Stage M did not tabulate each write subtype. An additional product veto requires its measured delta before activation. Cost by terminal class and the limits of these measurements are in the annex.

Every PR observes its own weighted-size limits. Binding logic belongs in `src/cpg/` modules and E logic in `src/resolution/js_ts/`. E0 moves the existing indirect pass before E1 adds a call there. The graph-changing slices bump the relevant cache versions.

### Seven runtime rulings for the fixture update

The integration worker's runtime evidence **stands in all seven cases**. The middle column replaces the disputed semantic expectation. The last column gives the product assertions and any accepted loss. A known excess edge is recorded as excess even though its label is NameOnly. A known missing transfer remains visible in the ledger. Neither can earn CORRECT credit.

| Row | Correct runtime outcome | Required product outcome |
|---|---|---|
| `t3-3` | On [7, 8] with no initializer the first call receives acc = 7 and value = 8. | Require both `arr → acc` and `arr → value` as builtin_callback. Keep h02a/b's explicit-seed negative for the receiver-to-acc edge. |
| `s1-17` | The call supplies [0, 1, 0, [1]] to the rest parameter. Element 1 enters `parts`. | Require one builtin_callback edge `arr → parts`. Replace the prohibition on that Def. |
| `s2-14` | `find` visits the hole with undefined. `map` skips it and never calls `skips`. | Keep the visits edge. The slot-zero table also proposes a skips edge, a known false positive on this input. Pin the absence in the runtime witness. |
| `s2-15` | The custom thenable calls `fail(reason)` only. The actual transfer is `reason → rejected.reason`. | D's fulfillment candidate is spurious. No D edge into rejected from the receiver. On an admitted Exact call to the user method, E requires `reason → rejected.reason` with call_nameonly. Scope the negative by source/label so it cannot ban E. |
| `s4-13` | The method supplies its local `payload` only. | Require the E target handler and `payload → handler.item` with call_nameonly. Record D's additional `box → handler.item` as a false positive. Neither edge is Exact. |
| `x3-9` | The installed reducer supplies acc = 0 and value = 7. The receiver element belongs in slot 1 only. | The static `map` table proposes slot 0 and misses slot 1. Pin both discrepancies and prohibit Exact on both. |
| `x5-8` | The first callback is the return value `visit`. Both call occurrences deliver to `visit.item`. `make.seed` receives only `input`. | Require the ordinary input edge and the second builtin_callback edge. The first builtin_callback edge is an accepted miss. Assert the two receiver occurrence spans separately. |

### The open classes

D's principle is a **potential delivery** from a represented receiver through an eligible callback and documented slot. A method spelling cannot prove dispatch, iteration, invocation count or which promise arm ran. E's principle is a **grounded callable value** obtainable from an admitted caller, at a stable formal and supported static path, within the two-lookups/eight-target bounds. Executing user code to produce or mutate that value, correlating guards after the union, or passing through another derived call supplies no extra witness. These principles cover the enumerated members in the D and E tables of the [row ledger](RULES-INPUT-NOTES.md).

The late D members are t5-2/3, s5-13/14/15/16/17/18 and x5-7/8. The late E members are t5-4/5, s5-19/20/21/22/23/24/25/26/27/28 and x5-9/10/11/12/13/14/15/16/17/18. Another species, decorator, factory or coroutine example instantiates the same boundary. A further D enumeration could change the rule by showing a wrong receiver/role **with the same admitted value and role proof**, or an existing, separately measurable receiver/value proof that distinguishes an accepted miss. For E it would take a stale or conflated target despite satisfying the admission, stability and path rules, or a bounded new terminal witness with measurable recovery and no derived grounding. A counterexample to a frozen rule or shared contract goes to its owner. Neither cap exhaustion nor an accepted loss establishes enumeration completeness.

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

Stage M's supplied decision is **GO for A, B, C, D and E** on build `06157400a3be`. M1 found ten dropped trailing-spread sites and 26 outside-span SB rows at seven sites (18 unproved, 8 graded chain, zero correct removals). M2 counted 482 admitted sites. M3 found zero WRONG Exact and 104 all-CORRECT SB sites. M4 counted 3,910 candidates and 98.5% typed library declarations. M5 counted 1,253 conversion candidates. M6 found 252 already traced cases, zero missing-rest first breaks and two handler registrations for lane 12. The census forecast passed. Actual parent credit still requires the deduplicated, all-CORRECT union or new traces on the merged head. [Stage M census](/Users/wesleyjinks/prism-evidence/param-binding/CENSUS.md)

## 7. Gates for the slices that run

| Gate | Statement |
|---|---|
| Oracle | every added or removed cross-function edge is graded by a grader independent of the product: added `exact` edges allow zero WRONG; name-only edges at most 5% WRONG, rate reported; only CORRECT rows earn credit. Transfers are graded by shape: formal binding (Use → Def, by runtime offset, runtime implementation behind TS overloads, TS `this` occupying no slot, constructors), literal leaf → read (Use → Use), call target (E: holder callers resolved by the checker, not by the census; a contrary declaration is WRONG, a missing witness UNDECIDED) and built-in callback (D: the library declaration and witnessed element/seed role, including a rest Def covering that slot). Each mode is checked against hand-labelled rows (a positive, a wrong target or slot, an undecided case) before it gates anything |
| Differential | only the slice's own classes of change; slice A's removals are each spread-misbound (proved-wrong or unproved) or chain-misbound; refusal accounting: an outgoing edge of a source the new-source refusal guards that reappears or disappears is listed with the reason, a seed change or a reachability change, its seed and its witness path before and after, in every slice (a D or E edge can connect a guarded source to an existing seed) |
| Counters | per slice, a listed set may move: call-resolution counters only in E2; return-flow certification (`return_flow_suppression_*`) in A to E when a binding it depends on is added or removed, each change reconstructed; the slice's own label counter in D. Every other movement fails |
| Benchmark | traced cases do not decrease in any class; gains and losses listed by case id |

The Oracle gate's grading uses lane 11's oracle: lane 9 adopts the measurement kit's oracle (owner,
2026-11-24), and until that oracle has merged the lane grades with its own checker, migrating when a bound
replay of its baseline agrees (lane 11 DESIGN section 6, G-adopt). The formats to write the question module
against are in `lane11-measurement-kit/FORMATS-PROVISIONAL.md` until lane 11's stage D writes `FORMATS.md`.

Check the seven corrected truths against the independent fixture evidence. A default-library signature alone cannot establish invocation on a fixture that demonstrably skipped the callback. Disclose a grader's UNDECIDED separately from a known runtime discrepancy. A contrary witnessed transfer earns no credit, and all checker-WRONG NameOnly rows remain in the published rate. The oracle self-test includes omitted vs explicit undefined initializer, rest capture, wrong receiver/slot and a callback not invoked. Grade all additions on the head, including B's rest elements and D's reducer seed, before crediting any site. Keep the supplied 5% denominator rule and zero-WRONG-Exact gate. No accepted loss waives an oracle gate.

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Slice thresholds | section 6 | none | at the census |
| 2 | Label of built-in callback edges | Adopted default. NameOnly(BuiltinCallback), badness rank 6 | Exact would need receiver and delivery proof | closed at stage D |
| 3 | `arguments`-object binding | out of scope | its own slice if a census shows demand | — |
| 4 | Handler-registration benchmark cases | re-measured here, handed to lane 12's follow-up | — | at M6 |
| 5 | The existing #434 plan | Adopted after M5 with anonymous identities and the bounded rules in §5 | A redesign goes through design escalation | closed at stage D |
| 6 | An added `exact` edge the grader leaves UNDECIDED | Adopted default. List each row and its missing witness, zero credit. Apply only the independently measured product predicates | Any further veto needs a costed product rule | closed at stage D |

## 9. Accepted losses and outstanding evidence

**120 distinct matrix rows** carry an explicit Stage D loss. An ID counts once even when both a missing and an excess transfer occur. The list includes passing negative controls for which a separate, identifiable runtime transfer remains outside the model. Each row's shape, required output and reason are in the corresponding entry of the [row ledger](RULES-INPUT-NOTES.md). Loss counts are fixture counts. Corpus rows with no checker credit have their own denominator.

| Matrix group | Loss count | Row IDs |
|---|---:|---|
| control | 0 | — |
| A | 15 | `a06e`, `a06f`, `a08a`, `a08b`, `t1-3`, `t1-4`, `t2-3`, `t3-1`, `t5-1`, `s1-1`, `s5-2`, `s5-3`, `x1-2`, `x3-1`, `x4-1` |
| B | 15 | `b03`, `b05a`, `b07`, `b09a`, `s4-4`, `s4-5`, `s5-6`, `x1-5`, `x1-6`, `x1-7`, `x2-4`, `x2-5`, `x3-5`, `x4-4`, `x5-3` |
| C | 5 | `s3-9`, `x1-8`, `x1-9`, `x5-5`, `x5-6` |
| D | 35 | `h02a`, `h02b`, `h05a`, `h05c`, `h06c`, `h07g`, `h08`, `h09a`, `h09b`, `h09c`, `h12a`, `t1-11`, `t1-12`, `t2-5`, `t2-6`, `t5-3`, `s1-13`, `s1-15`, `s2-11`, `s2-14`, `s2-15`, `s3-11`, `s3-12`, `s4-11`, `s4-12`, `s4-13`, `s5-14`, `s5-15`, `s5-17`, `x1-12`, `x3-9`, `x4-7`, `x4-8`, `x5-7`, `x5-8` |
| E | 50 | `e05`, `e06b`, `e06d`, `e08b`, `e11`, `t1-9`, `s2-21`, `s2-25`, `s3-15`, `s3-16`, `s3-20`, `s3-21`, `s3-22`, `s4-16`, `s4-17`, `s4-23`, `s5-19`, `s5-20`, `s5-22`, `s5-28`, `x1-19`, `x1-22`, `x1-23`, `x1-25`, `x2-11`, `x2-12`, `x2-13`, `x2-14`, `x2-17`, `x2-20`, `x2-22`, `x3-12`, `x3-14`, `x4-9`, `x4-10`, `x4-11`, `x4-12`, `x4-14`, `x4-15`, `x4-16`, `x4-18`, `x4-19`, `x5-9`, `x5-11`, `x5-12`, `x5-14`, `x5-15`, `x5-16`, `x5-17`, `x5-18` |

The ledger adjudicates all 40 input `ambiguous` rows, all 23 currently marked `unexecuted` and all seven disputes. The original RED record contained ten unexecuted rows. The implementer enables three h09 negatives after confirming their callback Defs. Seven retain external checks (a07b, a09b, b10c/d, h12b, h14b, e16b). Matrix review `5767ceb8` marked 13 more rows as model-boundary skips. All 13 have individual loss rulings. That review also removed unsatisfiable nonreachability assertions from x1-6 and x3-5. Their remaining positives stay in scope. On M3 all **73** UNDECIDED rows are enumerated with both endpoints and evidence reasons. Admission withholds 14 Exact and the ordinary C precision rule withholds 44 NameOnly, leaving 15 product edges at zero credit. The six WRONG NameOnly rows are a separate, measured exclusion. No C refusal is keyed to a grader verdict.

**Three unresolved fixture checks** keep their mandatory assertions: x1-4 (a repositioned fixed argument at an unimported global), x2-2 (super lookup and virtual dispatch) and x5-2 (a constructor returns a different receiver). First bind the site's actual resolution on base and head. If complying would require changing A or another resolver, report the mismatch to the controller. In the separate model-boundary skips, t5-1's patched iterator yields two values where frozen A assigns length one, and x2-4's default reads the uninitialised rest binding. Keep those runtime facts visible. No loss or skip waives the zero-WRONG-Exact differential.

The known unimported-export resolutions in c07/c08 remain pinned. For lane 8's I-guard, the trace's default Exact ceiling excludes doubtful edges. The supplied M4/M5 counts measure 3,910 D candidates and 3,782 proposed E bindings. Actual newly stopped trace paths are measured on lane 8's differential. The outgoing source guard can change reachability when any slice adds or removes a bridge. List each seed and its before/after witness. Tier-A, full corpus oracle results, lane 8's I-guard integration and both design reviews are separate receipts. The controller owns review dispatch (two rounds), shared-contract decisions and release.

## Change log

| Change on 2026-11-24 | Evidence and reason |
|---|---|
| Updated the status, selected approach in §4 and §5 slice briefs. Retained the three common rules and A's runtime-offset operation. | Stage M passed all five gates. The 425-row matrix supplies decisions unavailable to the tentative design. |
| Fixed B's rest threshold, spread-operand and dense-element rules. | b03/b09a separate unknown position from proved membership. t3-1, s5-7 and x1-5 distinguish offsets and element provenance. M2 counts 482 sites, 53 spread-only. |
| Qualified ordinary bare binding and recorded every uncredited M3 row. | The ordinary cross-file NameOnly predicate removes all six checker-WRONG rows at zero observed CORRECT cost. An indiscriminate UNDECIDED refusal has no such justification. |
| Fixed D's method/receiver, identity, roles, rest coverage, omitted-initializer seed and confidence order. Recorded the seven runtime rulings. | M4 counts 3,910 eligible and 10,114 receiver-absent sites. The recordings distinguish a real seed and rest delivery from false dispatch and unmodelled callback factories. |
| Recorded lane 8's published I-guard answer, matched both variants by doubt and kept the two merge gates. | The same-line Exact loan in h13/e14 requires an edge-local decision. A plan commitment is not an executed integration test. |
| Adopted #434's fallback, explicit stable declaration/path/default rules, grounding, depth, deduplication, persisted facts and E0/E1/E2 ownership. | M5 counts 1,253 conversions and 105 fanout sites with payload. The late D/E rows require explicit model boundaries. The current storage object can hold derived facts without increasing call_graph.rs. |
| Added Stage M results to §6, role and grading instructions to §7, closed local defaults 2/5/6 in §8 and enumerated losses/sentinels in §9. | Keep corpus cost, fixture truth, historical RED, unexecuted evidence, review approval and parent-gate credit independently auditable. |
| Added RULES-INPUT-NOTES.md and reconciled PLAN's matrix requirements, Stage D handoff, all affected slice briefs, cache parity, oracle and close-out checks. | Every row now has a disposition against named rules. The implementer can update and rerun changed assertions without inheriting superseded instructions. |
| Reconciled fixture review `5767ceb8` with the earlier RED receipt, and pinned the shared literal-leaf boundary. | The live matrix gained 13 skip markers, two narrower walk assertions and the label-adapter fix. The leaf producer's earlier-spread and Exact-singleton checks explain s1-1, s5-3 and s5-28 without changing lane 3's contract. |
| Distinguished writes before a property is selected from writes after a callable is copied, and recorded runtime transfers behind passing negative controls. | M4's 88 named candidates and M5's 8/152/6 body-destructure sites bound their respective populations. D and E1 measure incremental costs on the same keys. The matrix loss count includes concrete adapter, method, accessor and stale-binding omissions even where the decoy prohibition already passes. |
