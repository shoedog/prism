# Prism Tier 2 Plan A Follow-ups

> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

Tracked follow-ups intentionally deferred from the Plan A substrate review fixes.

> **MERGE STATUS (round 10):** both review lenses (codex correctness + claude/fable architecture)
> returned `MERGE VERDICT: APPROVE`; codex found zero findings. The substrate merged as the A3+A4+A7
> v1 gate. The single round-10 item (`Relation::SameLineDefUse → RecoveredDefUse` rename) landed before
> merge. Everything below is Plan B surface or a documented v1 precision limit — NOT a merge blocker.
>
> **The dominant Plan B item is the line-granular CFG-scoping / DFG-ref-visibility family.** Rounds 6–9
> each surfaced a real intra-function false negative in it (shared lines, nested functions, nested/long
> continuations, field-path ref visibility), each fixed Option-C-safely with a targeted degrade /
> fail-open / recovery arm. The contract already disclaims the residue (`NotReached` = "not reached within
> v1's traced scope," NOT proven absence). Plan B's **node/location-precise seeding + interprocedural
> chase** is the wholesale architectural fix — it should subsume the per-case patches, at which point the
> degrade/fail-open heuristics here can be simplified.

## MAJOR 4: `SinkResult.graph_node` Truncation Repair

Plan B follow-up, once `ReasoningSummary` is populated:

- Define whether reasoning evidence owns an embedded witness `GraphPayload` or whether `Evidence.graph` is the reasoning witness graph for reasoning-bearing results.
- Repair or clear `SinkResult.graph_node` when graph clipping or byte-budget truncation removes the referenced node.
- Add serialization tests covering clipped witness graphs so no `graph_node` points past the emitted `graph.nodes` array.

## MINOR 7: Assignment Propagation Byte-range Scoping

A2/A3 follow-up:

- Replace line-granular `AssignmentPropagation` with statement or byte-range scoped propagation.
- Cover same-line independent assignments such as `a = b; c = d` so taint from `b` cannot propagate to `c`.
- Reuse the per-call byte-range scoping approach already used by the PR #74 call handling where practical.

## MAJOR 2: Parameter seeds use the pure-taint fallback (over-approximate)

A parameter seed sits at the function-start line, which has no CFG statement node, so v1 falls back
to **pure taint** for that seed (`Trace.degraded = true`) — no CFG pruning of the parameter's flow.
This is an over-approximation (the conservative/safe direction for a defect finder, consistent with
v1's other over-approximations: line-granular propagation, field-insensitivity); the parameter still
reaches its body uses. CFG-precise parameter handling is deferred.

- Derive a CFG body-entry for parameter seeds — the first *executable* statement of the function body
  via the CFG/statement index — so the parameter's flow can be CFG-pruned like any other seed.
- Beware the dead-code trap: a naive "first variable line after the header" entry (an earlier fix
  attempt) lets a parameter reach past an early `return`. Add a param-seed fixture with an early
  `return` before first use, asserting the dead-code path is not reported as reached.

## Line-level `reachability_at` is best-effort (documented contract, not a defect)

The CPG has no `Call`→argument edges, so the `(file,line)` wrapper cannot bind a use to a specific
sink argument. `reachability_at` is therefore conservative (`Reached` only if *every* candidate use
on the line is reached) and may `NotReached` a line whose actual argument is tainted. The sound,
node-precise contract is `reachability_for_node`; Plan B resolves each sink to specific
`VarLocation`s and consumes the node API per argument. No fix needed — recorded so Plan B does not
build on the lossy wrapper.

## Deferred from the in-depth code-review (round 2) — MINOR / Plan-B / architectural

- **Vocabulary placement (#7):** `ReasoningReason`/`ReasoningWarning`/`Reachability`/`SinkResult`/
  `ReasoningSummary` currently live in `src/navigation/types.rs` (Tier-1's serialized-contract module)
  while `reasoning/` imports them — the dependency arrow opposes the data arrow. Only the
  `Evidence.reasoning` *field* must stay in navigation for byte-compat; relocate the *definitions* under
  `reasoning/` (navigation references them) **before Plan B serializes these shapes externally**, else it
  becomes a breaking change.
- **`Evidence` constructor (#8):** ten sites hand-write `reasoning: None`; the A7 scaffold guarantees more
  additive fields in Plan B. Add `Evidence::new(query)` / `Default` + struct-update now while sites are fresh.
- **Typed trace→Evidence fidelity (#9):** `Trace.warnings: Vec<String>` (vs structured `ReasoningWarning`),
  `Trace.degraded: bool` is global (degradation is per-seed — one unresolved seed poisons a multi-seed
  response with no attribution), and `cleansed_categories_for_source` returns `Vec<String>`. Return typed
  values (`Vec<SanitizerCategory>`, per-seed degradation); stringify only at the shape boundary; move the
  stringifier with the tracked A2 `taint.rs` → `sanitizers/` relocation.
- **`SinkResult` source attribution (#10):** `witness_graph_for_node` picks the first `BTreeMap`-order root
  that reaches the sink — deterministic but arbitrary under multiple seeds; "which source taints this sink"
  is unanswerable. Plumb `root` into `SinkResult`/witness selection before the shape freezes (Plan B).
- **`CfgScope` tri-state (#6):** `cfg_scope_for_seed` encodes "no CFG" as `Some(empty set)` (literal meaning
  "prune everything", safe only because `has_cfg` is checked first). Encode the tri-state once, e.g.
  `enum CfgScope { NoCfg, Degraded, Lines(set) }`.

## Round 3 (in-depth vs main) — disposition

**FIXED this round (commit alongside this doc):**
- **Same-line cross-function over-fire (was reported as MAJOR, real):** `reachability_for_node` used
  `dfg_forward_reachable`, whose same-line assignment propagation keys on `(file,line)` alone and so
  reaches a `Def` in a *different* function that shares a minified line — classifying a sink in another
  function as `BoundaryExited` off an unrelated boundary. Replaced with
  `CodePropertyGraph::forward_reachable_in_function` (the same function-scoped `taint_neighbors`
  traversal the BFS uses), unifying the two engines. Pinned by
  `test_forward_reachable_in_function_is_function_scoped` (probe: old reaches `["a:t:Def","b:c:Def"]`,
  new reaches `["a:t:Def"]`).
- **`Trace.boundary` duplicates (MINOR):** `Vec` → `BTreeSet<BoundaryEdge>` (derive `Ord`) so parallel
  DataFlow edges / multi-root traversal can't double-count Plan B's `InterproceduralBoundary` warnings.

**~~NOT a defect — BLOCKER did not reproduce~~ — CORRECTED IN ROUND 4: the BLOCKER is real.** The
round-3 dismissal was over-generalized and wrong. It is true that a *strictly* one-line function
(`def g(p): sink(p)`, `start_line == end_line`) gets no param `Def` and no boundary — but round 4
produced the right witness: a **multi-line** function whose **first body statement shares the signature
line** (`function g(p) { sink(p);` newline `log(p); }`). The later `log(p)` makes the param `Def@1`
register, the boundary `u → p Def@1` IS recorded, and `data_flow.rs`'s `ref_line == start` skip drops
the def→use edge to the same-line `sink(p)` use — so `forward_reachable_in_function(p Def@1)` reaches
only `p Use@2` and the real sink classifies `NotReached` for a path taint crosses (verified probe). The
param-binding bridge was **restored** (`same_line_same_path_uses`, seeded only from `start` in
`forward_reachable_in_function` — param-scoped, so sound and out of the BFS frontier/witness graph) and
pinned by `test_boundary_classification_same_signature_line_body_use`. See "Round 4" below.

- **One-line-callee taint under-approximation (CPG-construction limitation, out of scope):** the *symptom*
  the reviewer observed — taint into a one-line callee is silently dropped — is real, but it lives in
  **CPG construction** (one-line functions get no param `Def` / no arg→param edge), affects the production
  Taint slice equally, and pre-exists this work. Fixing it means changing `data_flow.rs`/CPG building,
  which is **not Option-C-safe** (would alter existing nav/diff-review output) and is a far larger scope
  than Plan A. Deferred as a CPG-modeling task, independent of the reasoning layer.

**Explicitly contracted for Plan B (verdict: "close OR explicitly contract before `taint_reaches`
consumes this substrate"):** Plan A intentionally ships these as the v1 substrate surface; their final
shape is determined by Plan B's `TaintedBy`/`SinkResult` contract, so Plan B's FIRST tasks add them
rather than Plan A guessing now:
- **Per-root reachability/witness API (MAJOR):** `reachability_for_node` unions boundaries across all
  roots and `witness_graph_for_node` picks the lowest-`NodeIndex` root. For a single-seed trace (the
  common case) this is exact; for multi-seed it is the safe over-approximation (says BoundaryExited/Reached
  more readily) but cannot answer "which source taints this sink." Plan B adds
  `reachability_for_node_from(root, …)` / `witness_graph_for(root, sink)` with the unioned forms as
  wrappers, and plumbs `root` into `SinkResult` (subsumes #10).
- **Node/location-precise seeding (MAJOR):** `taint_trace(&[(file,line)])` promotes every Variable on the
  seed line to an independent root — concretely demonstrated this round: seeding a minified line with two
  functions seeds the *other* function's locals, inflating `frontier_count`. Plan B adds a
  `taint_trace_locations(&[VarLocation])` / node-precise entry point (symmetric with the round-2
  node-precise sink fix), with the line API as a wrapper.

**Remaining round-3 MINORs (contracted, low-risk, Plan-B-adjacent):**
- **`sink_nodes_at` ownership:** its callee-name exclusion + prefer-incoming-DataFlow heuristics exist only
  because the CPG lacks `Call`→arg edges; move to a `CodePropertyGraph` query so the graph owner co-evolves
  it when that follow-up lands.
- **`sanitizer_supported` second source of truth:** a hand-maintained language list separate from the
  per-language recognizer arrays. Add a language field to `SanitizerRecognizer` and derive support (folds
  into the #9 typed-fidelity / `sanitizers/` relocation).
- **Double-nested JSON discriminant:** `Reason::Reasoning(ReasoningReason::TaintedBy{…})` serializes as
  `{"Reasoning":{"TaintedBy":{…}}}`. Additive today (no existing output carries `reasoning`); pick a flatter
  encoding before Plan B ships external consumers (folds into #7 vocab relocation).
- **Boundary-classification memoization:** `reachability_for_node` recomputes a forward closure per boundary
  per sink — O(sinks × boundaries × graph) in Plan B's witness loop. Precompute per-boundary-target closures
  once per `Trace` when Plan B's witness mode lands.
- **`Trace` invariant on pub fields / `node_of` fabricated `Location`:** the witness invariant holds by
  construction (the `seen` guard already hedges) and `node_of`'s `{file:"",line:0}` fallback is unreachable
  while the frontier is all Variables. Harden (private fields + accessors; `node_of -> Option`) when Plan B
  restructures the witness builder.

## Round 4 (in-depth vs main) — disposition

The round-4 adversarial re-review (explicitly tasked with breaking the round-3 dismissal) **succeeded**,
confirming the value of in-depth re-review. Both lenses independently confirmed the round-3 over-fire fix
is correct and introduces no new false negative, then broke the BLOCKER dismissal.

**FIXED this round:**
- **Same-signature-line param use (BLOCKER, real):** restored the param-binding bridge
  (`same_line_same_path_uses` seeded from `start` in `forward_reachable_in_function`). Witness:
  `function g(p) { sink(p);` newline `log(p); }` — boundary to `p Def@1` recorded, the `sink(p)@1` use
  edge-dropped by `data_flow.rs`'s `ref_line == start`, now recovered. Pinned by
  `test_boundary_classification_same_signature_line_body_use`. Sound and witness-safe: the bridge fires
  only from a boundary target (structurally a callee parameter), so it never back-flows `x = x + 1` and
  never enters the BFS frontier or witness graph.
- **Duplicate degraded-seed warnings (MINOR):** `cfg_scope_for_seed` was called per root inside the
  variable loop, pushing N identical warnings for an N-variable degraded line. Hoisted to per-`(file,line)`
  (roots filtered first, so a no-variable line warns zero times). Pinned by
  `test_degraded_seed_line_warns_once_per_line`.

**MAJOR — one-hop boundary closure is definitive `NotReached` (resolved by honest documentation, the
reviewer's accepted v1 option):** for `f → g(u) → h(x) → sink(x)`, only the `f→g` boundary is recorded;
the `g→h` arg→param edge is filtered as cross-function and the BFS never ran inside `g` to record a
`g→h` boundary, so the truly-tainted sink in `h` is `NotReached`. v1 does **not** chase taint transitively
through callee chains — that *is* Plan B's `taint_reaches` (the full interprocedural chase). Documented on
`reachability_for_node`: `NotReached` means "not reached within the seed function ∪ first-hop callee
boundaries," **not** "proven absence"; Plan B consumers must not suppress warnings off it. The transitive
worklist-over-functions chase is Plan B's core deliverable, not Plan A substrate.

**CPG-construction limitations (out of scope for Option-C-safe Plan A; affect the production Taint slice
equally — documented, not fixed here):**
- **Strictly one-line callee** (`def g(p): sink(p)`, `start_line == end_line`): no param `Def`, no
  arg→param edge, no boundary — taint into the callee is silently dropped at the CPG level.
- **Declaration-initializer param use on the signature line** (the round-4 "severed" variant
  `function g(p) { var x = p;` newline `sink(x); }`): verified by probe that `var x = p` on the signature
  line registers **no `p` Use node** (the control with `var x = p` on a *later* line does register it), so
  there is no node for the reasoning-layer bridge to reach and no `p → x` dependency at all. Distinct from
  the call-argument case (`sink(p)`), which *does* get a Use node and *is* fixed. Fixing this needs CPG
  construction changes (register signature-line declaration-initializer uses), not an Option-C-safe
  reasoning-layer change.

**Still contracted for Plan B (re-confirmed by both lenses as behaving-as-contracted):** per-root
reachability/witness API; node/location-precise seeding; `CfgScope` tri-state enum (#6 — the
`Option<BTreeSet>` + `has_cfg` sentinel is correct today but a footgun; the comment now spells out why the
empty-set means "unfiltered" only because `cfg_valid` short-circuits on `!has_cfg`); vocabulary relocation
(#7); a `SinkUnresolved` state analog to `SeedUnresolved` (so an unresolvable sink line is not conflated
with proven `NotReached`); boundary-classification memoization; `Trace` pub-field hardening.

## Round 5 (in-depth vs main) — disposition

Both lenses confirmed the round-4 fixes correct, then found the same-line def→use seam one
propagation hop deeper than the start-only bridge covered.

**FIXED this round:**
- **Same-line def-then-use false negative (BLOCKER, real):** `data_flow.rs` drops a variable's own
  same-line def→use edge generally (param `ref_line == start` at :234; regular `ref_line == def_line`
  "skip self-reference" at :293). The round-4 start-only bridge only patched the param position, so
  `var y = u; sink(y)` (main BFS frontier — the node-precise contract Plan B consumes) and
  `var q = p; sink(q)` (classification, one hop past the param) still dead-ended. Replaced the
  start-only bridge with a **uniform** function-scoped, same-path `Def → same-line-Use` arm in
  `taint_neighbors`, symmetric to the existing `Use → same-line-Def` arm, so both the BFS and
  `forward_reachable_in_function` recover the dropped edges at every hop. Pinned by
  `test_same_line_def_then_use_reaches_intra_function` and
  `test_boundary_classification_same_line_def_use_one_hop_deeper`. Adding edges only over-approximates
  (safe direction), so this cannot introduce a false negative; it removes them.
- **Minified two-function seed line CFG over-prune (MAJOR, unsafe FN):** `cfg_reachable_lines` seeds
  from the first statement only, so a minified line hosting two functions CFG-pruned the second's
  roots into a false `NotReached`. Added `cfg_reachable_lines_unioned` (unions over all statements at
  the line; the safe over-approximation — Statement nodes carry no function field) and switched
  `cfg_scope_for_seed` to it. Production `cfg_reachable_lines` is left byte-stable (Option C). The
  round-4 hoist comment that misstated CFG scope as a line invariant is corrected. Pinned by
  `test_cfg_reachable_lines_unioned_covers_all_statements`.
- **Silently-dropped unresolved seeds (MAJOR, observability):** a seed line resolving to zero Variable
  nodes now pushes a warning instead of vanishing. Pinned by `test_seed_with_no_variable_nodes_warns`.
- **Duplicate `(file,line)` seeds (MINOR):** deduped on entry (a `BTreeSet`), so a repeated seed no
  longer re-runs the BFS or re-pushes a degraded warning. Pinned in
  `test_degraded_seed_line_warns_once_per_line`.
- **`node_of` non-Variable fallback (MINOR):** now `debug_assert!`s (fails loudly in debug) before the
  conservative empty node, so a future non-Variable chain node surfaces instead of corrupting witness
  JSON silently.
- **`SinkResult.graph_node` referent (MINOR):** documented on the field (index into the witness
  `GraphPayload.nodes`, not the truncating `Evidence.graph`; truncation-repair is a Plan B follow-up).

**Finding 2 — bridge-precondition over-fire: DISSOLVED + documented (safe direction).** The reviewer's
structural critique (the start-only bridge fired from `b.to` without proving it is a parameter) targets
code that **no longer exists** — the uniform `taint_neighbors` arm replaced the start-only bridge, so
there is no param-only special path to mis-precondition. The residual over-fire the reviewer demonstrated
(`function g(p) { p = clean(); sink(p); }` classifies `BoundaryExited` even though `p` was reassigned —
verified by probe) is **not** the bridge: it is the may-taint analysis having **no strong-update/kill** on
reassignment, an over-approximation into the *indeterminate* `BoundaryExited` state — the safe direction
for a defect finder, consistent with the documented line-granular / field-insensitive stance. Strong-update
modeling is deferred (Plan B precision), not a Plan A soundness blocker.

**Contracted (re-raised, unchanged):** double-nested JSON discriminant (#7 — settle the serialized
tag/rename before Plan B's first external emission; additive today since no output carries `reasoning`).

## Round 6 (in-depth vs main) — disposition

Both lenses confirmed the round-5 uniform same-line arm closes the seam (augmented-assignment,
destructuring, chained-assignment probes pass at the documented granularity) and witness walk-back
terminates. They found a *different* CFG-scope mechanism and several now-or-never contract decisions.

**FIXED this round:**
- **Shared-line CFG over-prune (BLOCKER, unsafe FN within the `Reached` contract):** CFG-scope
  degradation was decided per `(file,line)` — it degrades only when *no* statement exists on the line.
  When two functions share a line and only one contributes a statement (e.g.
  `int a(){ int z=0; bar(z); } int b(int p){` with `sink(q)` in b's body on later lines, `has_cfg`
  true), b's param root inherited a's CFG scope and b's intra-function flow was pruned into a false
  `NotReached` (verified by probe). The round-5 union does not cover this (it only helps when both
  functions contribute statements) and the param-fallback does not fire (statement_at finds a's
  statement). Fix: when a seed line's roots span more than one function, degrade the whole line to
  pure taint — the safe over-approximation — since Statement nodes carry no function field to split the
  scope. Pinned by `test_shared_line_multifunction_degrades_not_false_negative`. Node/location-precise
  seeding (Plan B) restores per-function precision.
- **`Relation::AssignmentPropagation` overloaded (MAJOR, now-or-never):** it named both a cross-variable
  assignment (`x = y`) and a variable's own recovered def→use chain. Added `Relation::SameLineDefUse`
  for the latter, with a distinct witness `kind` string — done now while `reasoning` is `None` in every
  production path and the wire shape is unfrozen, because Plan B's strong-update kills assignment edges
  but never a variable's own def-use chain.
- **MINOR docs:** `cfg_valid` now documents the ordering precondition (the BFS records cross-function
  neighbors as boundaries *before* `cfg_valid`, so it only ever gates intraprocedural targets; Plan B's
  transitive chase must preserve that). `sanitizer_supported` now documents that it is a second source of
  truth parallel to the recognizer tables and must be kept in sync.

**Contracted for Plan B (re-raised; deliberate now-or-defer calls):**
- **Order-insensitive witness edge (MAJOR, safe-direction false positive — must land before Plan B
  serializes witnesses):** the uniform `Def → same-line-Use` arm has no column/byte ordering, so for
  `function h(u) { sink(y); var y = u; }` it synthesizes a `y Def → y Use` edge to the temporally
  *earlier* `sink(y)`, yielding `Reached` plus a witness edge for a flow that cannot occur. This is the
  documented **line-granular AssignmentPropagation** over-approximation (safe direction — over-reports
  reachability; `reasoning` has zero consumers today), but the witness corruption makes the byte/column-
  range fix (already the MINOR 7 byte-range-scoping follow-up) **mandatory before any `reasoning`
  witness is serialized for Plan B**, synthesizing def→use edges only to same-statement-or-later uses.
- **Function identity is a `(file, name)` string pair (MAJOR):** `node_file_fn` keys on the function
  *name*, so two same-named functions in one file (two classes' `handle()`, `impl A`/`impl B` methods,
  same-named JS function expressions) conflate — a cross-function DataFlow edge between them would
  traverse as intra-function instead of recording a `BoundaryEdge`, a false `Reached` with a witness
  spanning two unrelated functions. The same string-identity weakness the minified-line fixes addressed,
  one level up. Plan B must key `BoundaryEdge`/`Trace` attribution on CPG Function *node* identity (nodes
  carry ranges) rather than name strings, before more consumers accrete.
- **`Trace.degraded` is trace-global (MINOR):** one degraded seed among several marks the whole trace,
  with no per-root attribution. Folds into the Plan B per-root API / per-seed degradation (#9).

## Round 7 (in-depth vs main) — disposition

The convergence-check round. Both lenses confirmed the shared `taint_neighbors` primitive is the right
seam (no trace/classifier divergence constructible) and the additive `Evidence.reasoning` is build-safe,
then found two in-contract unsafe false negatives — one common (recursion), one that did not reproduce.

**FIXED this round:**
- **Recursive self-call drops the param boundary (BLOCKER, real, common):** a recursive call `f(…, x)`
  feeds f's own parameter; the arg→param edge has `next_fn == src_fn` (both `f`), so the name-keyed
  boundary check missed it, the param's signature line is not CFG-reachable from the call site, and
  `sink(u)` classified `NotReached` (verified by probe: `boundary_len=0`). Fix: treat a *parameter
  binding* — a Variable `Def` on a function's signature line, only ever written by an arg→param edge —
  as a boundary regardless of name match, so recursion (and same-name collisions) record a `BoundaryEdge`
  → `BoundaryExited`. Pairs the boundary and CFG-admit decisions structurally (addresses MAJOR 2's
  prose-only ordering). Pinned by `test_recursion_records_param_boundary_not_false_negative`.
- **Nested named function param scope (BLOCKER as filed — did NOT reproduce; hardened anyway):** probed
  `function outer(){ return function inner(p){` … — it already degrades (`statement_at` is `None` on the
  signature line, not the encloser's `return` as the report assumed) and reaches correctly. Made the
  safety *explicit and robust*: a function-signature seed line degrades to pure taint (so a variant where
  `statement_at` resolves to an enclosing statement can't slip through). Pinned by
  `test_nested_named_function_param_seed_not_pruned`.
- **Non-exhaustive `Relation` → wire-string match (MAJOR):** `witness_graph_for_node` had a
  `_ => "DataFlow"` wildcard that would silently mislabel a future `Relation` variant. Made exhaustive
  over `Option<Relation>` so a new variant is a compile error.
- **MINOR docs:** `ReasoningSummary.reachability` aggregation rule (worst-case-over-`per_sink`) defined;
  `cfg_reachable_lines_unioned` documented as inert on real CPGs (≤1 statement per line; the
  multi-function/signature degrade is load-bearing); `sink_nodes_at` callee-shadowing heuristic limit noted.

**Contracted for Plan B (re-raised):** classifier/memoization seam for `forward_reachable_in_function`
(introduce an opaque view struct before the memo lands behind a stable seam); function-node identity vs
`(file,name)` string (the recursion fix uses the parameter-binding signal, which does not depend on name
identity, but cross-function same-name DataFlow edges still want node-identity attribution in Plan B);
the boundary/`cfg_valid` fold into a single `classify_neighbor` step (partially done — the boundary
condition now pairs the function check with the parameter-binding check).

## Round 8 (in-depth vs main) — disposition

Convergence-verification round. Both lenses confirmed the round-7 parameter-binding boundary and
signature-line degrade are correct in direction (the only false-positive mode is safe `BoundaryExited`,
no parameter-binding miss exists today). They found one ordinary-code unsafe false negative.

**FIXED this round:**
- **20-line continuation-scan cap (BLOCKER, real, ordinary code):** `cfg_reachable_including_continuation`
  scanned back at most 20 lines to find a continuation line's enclosing statement, so a tainted argument
  on the 21st+ continuation line of a multi-line call (`sink(\n a0,\n …\n user\n)`) was CFG-rejected →
  `NotReached` (verified by probe: `in_frontier=false` for the arg 25 lines down). Parameterized the cap:
  the production `taint_forward_cfg` path keeps `DEFAULT_CONTINUATION_SPAN` (20) to stay byte-stable, the
  reasoning path (`cfg_valid`) passes `usize::MAX` so it scans to the enclosing statement however far.
  Pinned by `test_long_multiline_call_argument_reaches_not_false_negative`; production taint
  (`algo_taint_cve`) unchanged.
- **Multi-line-signature recursion (MAJOR 2 — fragility, pinned):** verified the recursion fix already
  works for multi-line signatures (`function f(\n flag,\n u\n){…}` → `BoundaryExited`, because param Defs
  are pinned to the function `start` line). Added `test_multiline_signature_recursion_records_boundary`
  and a comment at `data_flow.rs` documenting that `is_parameter_binding` DEPENDS on that pinning
  convention — so a future change to param-Def lines can't silently revive the false negative untested.

**Contracted for Plan B:**
- **`BoundaryEdge` kind discriminant (MAJOR 3):** three events collapse into one shape — cross-function
  arg→param, recursive self-call param, and the intra-function pseudo-boundary (a local declared on a
  one-line function's signature line, which `is_parameter_binding` flags; safe-direction `BoundaryExited`,
  not a false `NotReached`). `BoundaryEdge` is internal `Trace` state (not a frozen wire shape), so Plan B
  adds the `kind` when it builds `InterproceduralBoundary` counting — together with the structural
  parameter-ness flag (which cleanly separates a true param from a one-line-signature local and resolves
  both this and MAJOR 2's fragility).
- **`ReasoningSummary::aggregate` + multi-source `SinkResult` (MINOR):** implement the aggregation beside
  the type and leave room for multi-source attribution before the shape freezes (folds into the per-root
  API). **`Evidence::new`/`Default` constructor (#8):** still hand-edited at 7 sites. **`CfgScope` enum
  (#6)** and **`classify_neighbor` fold (MINOR 5)**: land before Plan B edits this path.

## Round 9 (in-depth vs main) — disposition

Both lenses refuted convergence with two distinct concrete missed-taint inputs in ordinary code; both
have clean reasoning-only fixes.

**FIXED this round:**
- **Nested-callback continuation under-reach (BLOCKER, ordinary code):** the continuation walk-back from a
  tainted argument after an inline callback (`sink(() => { stmt }, user)`) stopped at the callback body's
  statement (not in the outer `cfg_set`) instead of the enclosing `sink(` statement → `NotReached`
  (verified by probe). Replaced the bare `usize` span with a `ContinuationScan` policy: production stays
  `Production` (nearest-only, 20-cap, byte-stable); the reasoning path uses `ReasoningFailOpen` (unbounded,
  scan to ANY reachable preceding statement). Fail-open is the safe direction. Also subsumes the round-8
  span knob and the MINOR-1 "policy as bare usize" concern. Pinned by
  `test_nested_callback_continuation_reaches_not_false_negative`.
- **Loop-carried field-path false negative (BLOCKER, ordinary code):** `ast.rs::collect_path_refs` filters
  a field path's refs to lines AFTER the def (unlike simple paths, which collect all non-shadowed refs), so
  a loop-carried `o.data def@N → use@M (M<N)` edge never exists. Added a field-path recovery arm in
  `taint_neighbors` (`Def` with non-simple path → same-function same-path uses on ANY line; the
  back-edge-aware `cfg_valid` keeps the loop-carried one and prunes infeasible ones). Corrected the
  `ast.rs` comment that falsely claimed field paths match simple-path scoping. Production DFG path left
  byte-stable (Option C). Pinned by `test_loop_carried_field_path_reaches_not_false_negative`.

**Reverted — MAJOR 1 was a false premise:** the reviewer proposed a `debug_assert` tripwire for "DataFlow
edges only connect Variables." But `test_taint_trace_skips_non_variable_dataflow_neighbors` shows the BFS
*intentionally* skips non-Variable DataFlow targets (a Statement-mediated edge is not a taint hop) — the
silent skip is tested behavior, not an invariant violation. Replaced with a comment pointing at that test.

**Contracted for Plan B:**
- **MAJOR 2 — continuation walk-back has no function-boundary stop:** the unbounded fail-open scan is safe
  today because callers gate it, but Plan B's contracted multi-function `cfg_set` (callee-chain chase) would
  let a statement-less line inherit reachability from a *different* function. Plan B adds the
  `function_starts_at` stop together with the multi-function `cfg_set` — NOT now, because today it would
  break the nested-callback fix (the callback's function-start sits between the argument and the enclosing
  `sink(`). The two are coupled and land together in Plan B.
- **MINOR 2 — lossy `reachability_at` has better API affordance than the sound node API:** its `(file,line)`
  signature is exactly what an MCP handler receives. Plan B's handler must consume `reachability_for_node`
  per-argument, not the lossy line wrapper; rename/`#[doc(hidden)]` when the handler is written.
- **MINOR 4 — `src/cpg/tests.rs` is ~1.7k lines (>600 rule):** split the trace/taint-trace tests into their
  own module before Plan B adds more. Test-org cleanup, not blocking.
