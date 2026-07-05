> **Status: SHIPPED — PR [#165](https://github.com/shoedog/prism/pull/165), merged 2026-07-04 (main 4f6455a).** As-executed brief incl. the folded codex xhigh spec review ([M1] multi-line-arg calls have no Step-5b edge — out-of-scope + pinned; [M2] CFG bypass keyed `depth == 0`, not root-fn identity — the mutual-recursion hole; [M3] staging re-cut so `Relation::CallDescent` lands in Stage A; [MIN1] first-enqueue depth-lock = documented v1 loss; [MIN2] companion library assertion pins the negative fixture's `r6_single_owner` kind; [MIN3] gate memoization). **As-shipped deltas beyond this text (whole-branch review + two fix waves):** (1) **Bypass-proven `Sanitized`** — the whole-branch gpt-5.5 xhigh review found first-parent dedup could hide an unsanitized sibling route at a descent confluence (`g(safe); g(raw)` → false Sanitized); shipped fix = `sanitizer_bypass_exclusions` (every first-parent-tree hop passing the P10 byte-reconstruction proof; TREE-scoped — the implementer correctly generalized the controller's chain-scoped prescription, which mis-verdicts the both-sanitized case) + `taint_trace_nodes_excluding` re-walk through the ONE shared walk core; strengthens P10 path-proven → ALL-paths-proven and closes the pre-existing intra-function first-parent merge hazard. (2) **Verdict-classified residual** — the fix-delta re-review found the re-walk tested frontier membership, collapsing the 3-valued verdict space (an Exact-sanitized + NameOnly-raw pair into the same callee sink reported `Sanitized`; truth `BoundaryExited`, which outranks it); shipped fix = classify the re-walk with `reachability_for_node_from_ordered` (Reached→bypass w/ displayed bypass chain; BoundaryExited→BoundaryExited w/ raw-boundary output shape; NotReached→Sanitized), and `descent_depth` recomputed from the DISPLAYED chain. (3) Stage A interim sanitizer guard (function-identity window skip) replaced in Stage B by relation threading via the shared `shape::window_relations` recovery. (4) Fixture plan executed as rework-in-place: `taint_boundary_negative` became the true negative (unknown-receiver `r6_single_owner`@0.6, live-verified) and `taint_cross_function_positive` was added as the armed flip. Known accepted overage: `taint_reaches.rs` at 631 lines (>600 guideline; module split = follow-up). Fix-wave narrative: `2026-07-04-prism-p14-fixwave-report.md` (committed by wave 2, relocated here).
>
> **Verification record (controller-run, final @ merge):** `cargo test --features mcp` = 30 targets / 3199 passed / 0 failed / 1 ignored; eval pytest 557; matrix all-ok (1 pre-existing `expected_gap`); fmt clean; no new warnings; live probes — flip `Reached` + `descent_depth: 1` + `CallDescent` witness edge + no boundary warning; NameOnly negative and 3-hop depth chain `BoundaryExited`; P10 pair byte-identical; wave-1 repro (`g(safe); g(raw)`) `Reached` w/ empty `sanitized_by`; wave-2 single-root repro (Exact-sanitized + NameOnly-raw) `BoundaryExited` w/ honest Cleansed + InterproceduralBoundary warnings; two-root control (only sanitized root seeded) correctly `Sanitized`. Perf: warm `taint_reaches` on the prism corpus 0.59–0.64 s (flat vs pre-P14). Failing-first evidence: T-F1/T-F4 and W1a/W2a verified failing against their pre-fix commits; the forced-relation sanitizer test proves the CallDescent window skip on a window that genuinely matches otherwise. No cache bumps (walk-time change).

# Task P14 — Interprocedural taint descent (Exact-gated, depth-bounded)

Repo: /Users/wesleyjinks/code/slicing (branch off main @ 3d46a24+; isolated worktree).
Plan item: P14 in docs/analysis/prism-llm-and-accuracy-plan.md. **The plan's anchors are
corrected by grounding (§0) — this brief is the design of record.** Locate code by the
SYMBOLS named here; line anchors are hints that rot.

> **SPEC-REVIEW FOLD (codex gpt-5.5 xhigh, fix-then-ship, 0 BLOCKER; findings folded
> below as binding text, marked [M1..MIN3]).** Summary: [M1] multi-line-arg calls have
> NO Step-5b edge (arg lookup at `site.line`, build.rs ~:923) — out of scope, pin it;
> [M2] CFG bypass keys on `depth == 0`, NOT root-fn identity (mutual recursion
> re-enters the root fn at depth 2); [M3] staging re-cut (`Relation::CallDescent` +
> exhaustive matches + parentage = Stage A); [MIN1] first-enqueue-wins can lock a
> higher depth — documented v1 loss; [MIN2] the NameOnly negative fixture needs a
> companion library assertion of the resolution kind; [MIN3] memoize the gate per
> recovered CallSite identity + target. Clean buckets verified by review: Exact-target
> matching sound (multi-target Exact exists — assert "param owner ∈ Exact targets",
> never "singleton Exact"); edge-local refactor behavior-identical absent descent;
> P10 downgrade ordering already pre-witness-gate; Evidence serialization additive-safe
> through all P12 modes + pruner.

## 0. Grounding corrections (verified 2026-07-04 against main 3d46a24)

1. **[ANCHOR ROT] arg→param edges are built in `src/cpg/build.rs` Step-5b**
   (`collect_step5b_edges`/`step5b_edges_for_caller`, ~build.rs:829-965), NOT
   data_flow.rs:277 (that is a comment). They are per-arg, field-sensitive (full access
   path + base fallback), positionally matched, and emitted as bare
   `CpgEdge::DataFlow` — **the edge carries NO CallSite and NO confidence**
   (explicit comment ~build.rs:869: "DataFlow carries no confidence/kind and taint
   consumes it directly").
2. **Step-5b wires edges for Exact AND ordinary NameOnly resolutions.** It resolves via
   the GATED `cg.resolve_call_site(site)` (= `filter_func_value_fanout(
   resolve_call_site_full(..).resolved)`, resolution.rs ~:2260) and drops ONLY
   `ResolutionKind::R6MultiOwnerCandidate` (~build.rs:872). Nav-only kinds
   (PropertyAccess, FrameworkEntry, CallbackRegistration) never become CallSites so
   never reach Step-5b. **Consequence: a CrossFunction boundary hop at trace time may be
   backed by Exact OR NameOnly, and the walk cannot tell from the edge — the descent
   gate must re-derive confidence from the retained `call_graph`.**
3. **No callee-return→caller-LHS variable edges exist** (Step 5's
   `CpgEdge::Return(confidence)` connects Function NODES only). Taint has no way back
   OUT of a callee. **v1 is descend-only**; return-flow is an explicitly-declared
   non-goal (§6).
4. **The walk** (`src/cpg/trace.rs`): per-root forward BFS (`VecDeque` + `enqueued`
   BTreeSet) over petgraph `CpgEdge::DataFlow` edges plus two synthetic relations
   (`Relation::{DataFlow, AssignmentPropagation, RecoveredDefUse}`,
   `taint_neighbors` ~trace.rs:463). TWO near-identical walk copies:
   `taint_trace` (~:126, line-seeded, tests-only consumer) and `taint_trace_nodes`
   (~:243, node-precise — **the one `taint_reaches` calls**, taint_reaches.rs:49). The
   boundary handler is DUPLICATED (~:211-224 and ~:317-327): if
   `next_fn != src_fn` (node-level identity from `node_file_fn` — every Variable node
   carries `(file, function, function_start_line)`) or `is_parameter_binding_from`
   (recursive self-call check), it records a `BoundaryEdge{root, from, to, kind}` and
   `continue`s — the callee param node is NOT enqueued and gets NO `parents_by_root`
   entry. `src_fn` is FIXED per root.
5. **CFG scoping is seed-scoped** (`cfg_scope_for_seed`; `cfg_valid(&src_fn.0, line,
   &cfg_scope, has_cfg, next)`), and the precondition comment at ~trace.rs:695-701
   explicitly warns: relaxing the boundary `continue` makes a caller-seeded `cfg_set`
   prune every callee line. Descended nodes MUST bypass the seed's CFG scope
   (pure-taint in callee bodies — fail-open, consistent with the existing degraded
   fallback).
6. **Verdicts live in `src/reasoning/shape.rs`**, not trace.rs:
   `reachability_for_node_from_ordered` (~shape.rs:84-111): `Reached` iff sink ∈
   `frontier_by_root[root]`; else `BoundaryExited` iff some boundary edge's `to` is the
   sink or reaches it via `forward_reachable_in_function_ordered`; else `NotReached`.
   `Sanitized` is a downgrade applied ONLY in `witness_mode`
   (taint_reaches.rs ~:180-201) when `sanitized_hits_on_chain` proves a
   contiguous same-assignment transition on the witness chain (P10). Severity
   `Reached(0) > BoundaryExited(1) > Sanitized(2) > NotReached(3)`; the match is
   exhaustively spelled per-variant to compile-break on a new variant
   (types.rs ~:70-100) — **do not add a Reachability variant**.
7. **Witness chain** = `Vec<NodeIndex>` from the first-parent tree
   (`parents_by_root: BTreeMap<(root,node),(parent,Relation)>`), rebuilt sink→root by
   `witness_chain_for` (shape.rs ~:251). `sanitizer_walk::sanitized_hits_on_chain`
   walks `chain.windows(2)` doing SINGLE-FILE byte-span reconstruction (use-node inside
   the assignment's RHS/data-arg span; def-node byte-EXACTLY the same assignment's
   target span). A cross-function window (arg in caller → param in callee) cannot match
   structurally today, but the walk must not be left to accidental non-matching —
   descent windows must be explicitly skippable (§3 S4).
8. **[ANCHOR ROT] The "reasoning-confidence hole" at types.rs:172-181 has moved**; no
   confidence field exists in reasoning types. `ReasoningReason::TaintedBy.path_proven`
   is wired but hardcoded `false` at its only production site (taint_reaches.rs ~:109,
   frontier mode). **Do NOT overload `path_proven`** for descent.
9. **Confidence recovery pattern already exists**: `is_parameter_binding_from`
   (~trace.rs:428-461) maps a DataFlow hop back to its CallSite via
   `self.call_graph.callers.get(callee_name)` matched on `site.line == call_line` +
   caller `(file, name, start_line)`. `CodePropertyGraph` retains
   `pub call_graph: CallGraph` (build.rs ~:107) — available at trace time.
10. **The two taint engines are independent** (slicer `algorithms/taint.rs` →
    `taint_forward_cfg` over the separate `self.dfg`; reasoning → trace.rs over
    `self.graph`). P14 touches ONLY the reasoning path. The diff-driven slicer is out
    of scope and must be byte-identical.
11. **Fixtures**: `eval/fixtures/python/taint_boundary_negative` is the armed
    flip-to-`Reached` target (its expected.toml says so). There is NO true
    must-stay-BoundaryExited fixture — this task creates it. Matrix taint probes
    whitelist expect keys `{reachability, warning_kinds_present, sanitizers_present,
    frontier_count_min}` (eval/tier_a/matrix.py ~:92).
12. **Perf**: no microbench exists; the cold-build tier-a arc is BUILD-time and P14 is
    TRACE-time only (Step-5b untouched) — cold-build is trivially unaffected. The perf
    surface is per-call `taint_reaches` latency (frontier growth into callees).

## 1. Design overview

**One sentence:** at a CrossFunction boundary hop whose arg→param edge is backed by an
**Exact** resolution to exactly the callee the param node belongs to, ENQUEUE the param
node (with parent, for the witness chain) instead of recording a boundary — bounded by
descent depth, with the callee body walked CFG-unscoped; everything else (NameOnly-backed
hops, unresolvable hops, depth exceedance, recursion) stays a recorded `BoundaryEdge`
exactly as today.

**Doctrine compliance:** taint is an asserted-finding consumer — nothing below Exact
feeds it (plan §3a; consumer-visibility doctrine). NameOnly-backed Step-5b edges exist
in the graph but the walk must NOT cross them. Registration-grounded candidates never
produce Step-5b edges (grounding §0.2) so no special case is needed. **A false
`Reached` is the unsafe failure direction; a missed descent (stays `BoundaryExited`) is
safe.** Every ambiguity in the gate resolves to NO-DESCEND.

### S1 — Shared walk core + edge-local function identity

The boundary logic is duplicated across `taint_trace` and `taint_trace_nodes`
(doctrine-6 risk — P11's query-side drift class). Extract ONE shared neighbor-processing
step (fn or closure) used by both walks. Within it, replace the per-root fixed `src_fn`
comparison with the **edge-local** comparison `next_fn != node_fn` where
`node_fn = node_file_fn(node)` (each Variable node carries its own function identity —
after a descent the callee body walks naturally, and deeper boundary hops are gated
again). The root's fn is still needed for the CFG scope decision (S3) — keep it, but
the boundary test itself is edge-local.

State added to the walk: `depth: BTreeMap<NodeIndex, usize>` per root (root = 0; a
descended param node = parent's depth + 1; intra-function neighbors inherit parent's
depth). Determinism: BTree* everywhere, and neighbor iteration order is already
deterministic.

### S2 — The descent gate (ONE implementation)

New helper on the CPG (near `is_parameter_binding_from`, same style):

```rust
/// Some(..) iff the DataFlow hop from `from` (arg, caller fn) to `to` (param Def,
/// callee fn) is backed by an Exact resolution to exactly the function that owns `to`.
fn descend_target(&self, from: NodeIndex, to: NodeIndex, rel: Relation) -> Option<DescentInfo>
```

Rungs (ALL must hold; any failure → None → boundary as today):
1. `rel == Relation::DataFlow`; `to` is a `Variable{access: Def}` whose
   `(file, function, function_start_line)` differs from `from`'s (cross-function).
2. CallSite recovery: `call_graph.callers.get(&to.function_name)` filtered by
   `site.line == from.line && site.caller.{file,name,start_line} == from's fn identity`
   (the `is_parameter_binding_from` pattern). **Exactly one matching CallSite**; zero or
   ≥2 (same-line same-callee-name double call) → None (counted, §S5).
3. Resolve THAT site via the gated resolver `call_graph.resolve_call_site(site)` (the
   same resolver Step-5b used — agreement by construction). Descend iff the resolved
   set contains a callee with `confidence == ResolutionConfidence::Exact` whose
   `target` FunctionId `(file, name, start_line)` == the param node's owning function
   identity. **Target-matched, not merely "resolves Exact to something"**; a NameOnly
   match to the same target → None.
4. Depth: `depth[from] + 1 <= MAX_TAINT_DESCENT_DEPTH` (const, = 2; rationale: each hop
   is Exact-proven; the flip fixture needs 1; >2 chains are rare and this is the perf
   valve). Exceeded → None (counted distinctly, §S5).
5. Recursion/self-call (`is_parameter_binding_from` true, same function identity) stays
   a `SelfFunctionParam` boundary — **no descent for self-calls in v1** (node dedup
   makes it near-moot; keeping it out avoids witness-cycle reasoning).

On Some: enqueue `to`, set `parents_by_root[(root, to)] = (from, Relation::CallDescent)`
(§S4), record depth, and do NOT insert a BoundaryEdge (a descended hop is not a
boundary; the InterproceduralBoundary warning must not fire for it). On None: exactly
today's behavior (BoundaryEdge + continue).

[MIN3] Memoize the recovery+resolution per walk: `BTreeMap<(CallSite identity — file,
caller fn identity, line, callee name), Option<resolved target + confidence>>`; the
depth check stays outside the memo. Step-5b emits multiple edges per site (field +
base), so the same site is consulted repeatedly.

[MIN1, documented v1 loss] First-enqueue-wins parentage/depth means a shared callee
param first reached at depth 2 blocks a later 1-hop route from descending further.
Deterministic and SAFE (missed descent only). Do not implement depth re-relaxation in
v1 (parent rewrites would perturb witness trees); document at the walk state and add
the scenario as a deterministic test pinning the current (lossy) behavior.

[M1, out of scope — pin it] Multi-line calls (`g(\n  user\n)`) have NO arg→param
edge at all: `CallSite.line` is the call node's start line (ast.rs ~:5603) while arg
Use nodes keep the token line, and Step-5b looks args up at `site.line` (build.rs
~:923) — the gate never sees such calls because the edge doesn't exist; verdicts stay
whatever they are today (typically NotReached). DO NOT extend Step-5b in this task.
Add a library test pinning the multi-line shape's current verdict with a comment
naming the Step-5b arg-lookup limitation (follow-up queue item).

### S3 — CFG scoping for descended nodes

[M2] `cfg_valid` applies ONLY to nodes at `depth == 0` (never descended). Keying on
root-function IDENTITY is WRONG: under mutual recursion `f → g → f` (each hop Exact,
depth 2) the re-entered `f` nodes share the root's function identity and would be
pruned by the SEED's CFG scope. Every node reached across any `CallDescent` hop is
walked pure-taint (fail-open), even when it re-enters the root's function. Test the
mutual-recursion shape. Update
the PRECONDITION comment at `forward_reachable_in_function_ordered`/`cfg_valid`
(~trace.rs:695-701) — it currently warns against exactly this change; rewrite it to
state the new invariant (seed-fn-scoped CFG refinement; callee bodies unscoped by
design). `forward_reachable_in_function_ordered` (used by shape.rs boundary
classification) is function-scoped by construction and needs no change — verify with a
test that a non-descended boundary's classification is unchanged.

### S4 — Witness, relations, and the P10 sanitizer contract

- New `Relation::CallDescent` variant. Enumerate EVERY consumer of `Relation`
  (`ordering_admits*` checks `rel != RecoveredDefUse`; `is_parameter_binding_from`
  checks `rel != DataFlow` — a descent hop must still be recognized where semantics
  demand "this is a DataFlow-class edge"; the shaper labels witness-graph edges from
  the recovered relation, shape.rs ~:318). The witness `GraphPayload` edge kind string
  for descent hops is `"CallDescent"` (precedent: `"SanitizedBy"` — pin it in an MCP
  tool test like tools_reasoning.rs's edge-kind pins).
- **Sanitizer walk (P10 contract):** `sanitized_hits_on_chain` must SKIP windows whose
  connecting relation is `CallDescent` — a descent window is arg→param by construction
  and can never be an `x = sanitizer(y)` same-assignment transition; skipping
  explicitly removes the same-file coincidental-byte-span risk. This requires threading
  the per-window relation (recovered from `parents_by_root`, as the shaper already
  does) into the sanitizer walk — change its signature; do NOT re-derive relations
  inside sanitizer_walk from bytes.
- **P10 pins preserved verbatim**: `taint_sanitized_current` = Sanitized (+Cleansed,
  sanitizers_present), `taint_sanitizer_bypass` = Reached (+Cleansed,
  sanitized_by empty). NEW cross-function sanitizer tests (§4, T7/T8) pin the
  interaction: sanitize-then-pass (caller sanitizes, passes `safe` to callee that
  sinks) → the sanitizer transition is INTRA-function in the caller and remains
  provable → `Sanitized`; pass-then-sanitize-bypass (callee sanitizes its param but
  sinks the RAW param) → `Reached`.
- `prune_graph_to_reasoning` (mcp/output.rs ~:480): CallDescent edges are ordinary
  backward-ancestor edges (arg is the parent of param) — verify the pruner keeps the
  descended path (test), no special-case expected.

### S5 — Evidence surface + telemetry (additive only)

- `SinkSourceResult` gains `#[serde(skip_serializing_if = "is_zero")] descent_depth:
  usize` (max descended hops on the winning witness chain; 0 = intra-function —
  omitted on the wire, byte-compatible for all existing outputs). No other Evidence
  shape changes. **Do not touch `path_proven`.**
- Trace-level counters surfaced through the existing warnings/telemetry path (NOT
  user-facing warnings): `descents_taken`, `descent_blocked_name_only`,
  `descent_blocked_no_unique_site`, `descent_blocked_depth`. Where the reasoning layer
  already aggregates boundary warnings (`boundary_warnings`, taint_reaches.rs ~:451;
  `scope_honesty`), keep BoundaryExited warnings for non-descended hops UNCHANGED.
- CLI: no new flags. Depth is a const.

## 2. Scope: files expected to change

`src/cpg/trace.rs` (walk core, gate, depth), `src/reasoning/shape.rs` (relation
labeling), `src/reasoning/sanitizer_walk.rs` (window-relation skip),
`src/reasoning/taint_reaches.rs` (chain relations → sanitizer walk; descent_depth),
`src/reasoning/types.rs` (SinkSourceResult additive field), `src/mcp/output.rs` +
`tools_reasoning.rs` (edge-kind pin test; pruner test), `tests/reasoning/*`,
`eval/fixtures/python/*` (§4). **NOT expected**: `src/cpg/build.rs` (Step-5b untouched),
`src/data_flow.rs`, `src/algorithms/taint.rs`, cache versions (**NO cache bump** — the
CPG serialized shape is unchanged; descent is walk-time. If you believe you need a cache
bump, STOP and escalate).

## 3. Fixtures (eval/fixtures/python/)

- **Flip + rename**: `taint_boundary_negative` → `taint_cross_function_positive`
  (same 2-function source; expect `reachability="Reached"`, and REMOVE the
  `InterproceduralBoundary` warning expectation — descended hops fire no boundary
  warning). Update its comment header to record the P14 flip (its own text mandates
  this).
- **NEW `taint_boundary_negative`** (the true negative): sink inside a method reached
  via an UNKNOWN-receiver call that resolves single-owner **NameOnly** (e.g.
  `class A: def m(self, p): sink(p)` + `obj.m(user)` where `obj`'s type is not
  recoverable) → Step-5b wires the edge, the gate must refuse → expect
  `reachability="BoundaryExited"` + `warning_kinds_present=["InterproceduralBoundary"]`.
  VERIFY while writing it (against the built binary) that resolution is NameOnly
  (`prism nav callees` score 0.6) — if the receiver accidentally resolves Exact the
  fixture is not pinning what it claims. [MIN2] The taint fixture schema cannot assert
  resolution kind, so ALSO add a library-level companion test (tests/reasoning/) that
  asserts the fixture-shape call resolves `NameOnly`/`r6_single_owner` via the call
  graph — if future receiver-typing work flips it Exact, that test fails loudly and
  the fixture must be re-shaped (note the hazard in the fixture comment header).
- **NEW `taint_descent_depth_bound`**: chain `f → g → h → sink` of Exact calls (3
  hops, exceeds MAX=2) → `BoundaryExited`. Pins the depth valve.
- Keep `taint_reach_positive`, `taint_frontier_only`, `taint_sanitized_current`,
  `taint_sanitizer_bypass` UNMODIFIED (byte-identical expected.toml).

## 4. Tests (TDD — failing-first where feasible; library level in tests/reasoning/)

- T1 `cross_function_sink_reached_via_exact_descent` — rewrite of the existing
  `cross_function_sink_reports_boundary_exited_and_warning` (flip; keep the old name
  test DELETED, not weakened — the fixture pin covers the old behavior's absence).
- T2 `name_only_backed_hop_stays_boundary_exited` (the doctrine pin; unknown-receiver
  single-owner NameOnly shape).
- T3 `descent_depth_bound_yields_boundary_exited` (3-hop Exact chain).
- T4a (Stage A) `descended_parent_chain_crosses_functions` — `parents_by_root` links
  param→arg with `Relation::CallDescent`; per-root depth recorded correctly.
- T4b (Stage B) `descended_witness_surface` — witness graph edge kind `"CallDescent"`;
  `descent_depth == 1` on the result.
- T4c (Stage A) `mutual_recursion_reentry_is_not_cfg_pruned` — `f → g → f` all-Exact,
  depth 2: re-entered root-fn nodes are admitted (M2 pin).
- T4d (Stage A) `multi_line_call_shape_pins_current_verdict` — the M1 out-of-scope pin.
- T4e (Stage A) `shared_callee_first_enqueue_depth_lock_is_deterministic` — MIN1 pin.
- T5 `self_call_param_stays_boundary` (recursion pin — SelfFunctionParam unchanged).
- T6 `two_same_line_same_name_calls_do_not_descend` (gate rung 2: no unique CallSite).
- T7 `sanitize_then_pass_across_descent_is_sanitized` (caller: `safe=html.escape(user);
  g(safe)`; callee sinks param) — Sanitized survives descent.
- T8 `callee_body_sanitizer_bypass_stays_reached` (callee: `safe=html.escape(p);
  sink(p)`) — raw-param sink stays Reached; and the sanitizer window walk, given
  CallDescent-relation info, does NOT evaluate the descent window as a transition.
- T9 shape.rs: non-descended boundary classification byte-identical (existing tests
  must pass unmodified — enumerate: cross-fn NameOnly, forward-reachable-from-boundary
  sink).
- T10 MCP: edge kind `"CallDescent"` in the pruned graph payload; `descent_depth`
  omitted-when-zero (existing golden/nav_compat outputs BYTE-IDENTICAL — hard gate).
- T11 frontier mode: descent applies there too (frontier grows into callee); the
  `taint_frontier_only` fixture (single-function) unaffected.
- Determinism: run T4 twice, identical output.

## 5. Acceptance & validation criteria (gates before review)

1. All of §4 green; full `cargo test` (all targets) green; `cargo fmt` clean; no new
   warnings; `cargo test --features mcp` green.
2. `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (after release build in
   the SAME worktree): the renamed positive + 2 new fixtures pass; ALL other rows `ok`
   (1 tracked `expected_gap` for rust/nested_test_module_glob_gap is pre-existing);
   `uv run pytest` in eval/ green (excluding the known adoption-harness environment
   failures, if any — report them, don't chase).
3. Live acceptance probes (report outputs verbatim):
   - flip fixture: `prism nav taint-reaches --repo eval/fixtures/python/taint_cross_function_positive --source app.py:6 --sink app.py:2 --format json` → `reachability == "Reached"`, `descent_depth == 1`, witness graph contains a `CallDescent` edge, NO InterproceduralBoundary warning.
   - new negative: same command shape → `"BoundaryExited"` + boundary warning present.
   - P10 pins: both sanitizer fixtures byte-identical verdicts.
4. Perf sanity (report, not gate-blocking): time `prism nav taint-reaches` on one large
   corpus seed before/after (e.g. a prism-repo source with `--source src/main.rs:<line>`)
   — order-of-magnitude regression = STOP and escalate.
5. Byte-compat: all frozen nav_compat goldens UNMODIFIED; `--format json` diff-review
   outputs untouched (descent only changes taint_reaches Evidence, additively).

## 6. Non-goals (do NOT implement)

Return-flow (callee→caller taint via return values — no edges exist; follow-up item);
NameOnly or registration-grounded descent; recursion descent; per-callee CFG scoping;
new CLI flags; new Reachability variants; changes to the diff-driven slicer
(`algorithms/taint.rs`) or Step-5b edge construction; cache bumps; `path_proven`.

## 7. Guardrails & edge cases (implementers: read twice)

- **False `Reached` is the unacceptable failure.** Every gate ambiguity → no-descend.
  If you find a path where a NameOnly-backed hop can be walked, that is a BLOCKER —
  fix before proceeding.
- Same-line multiple calls (`g(user); g2(user)` on one line; `g(g(user))` nested):
  CallSite recovery matches on line — nested same-name self-composition
  (`g(g(user))`) has ONE callee name and possibly 2 sites... if `callers` records two
  sites on that line for `g`, rung 2 (exactly-one) refuses — write the test (T6 covers
  the two-call case; add nested if representable).
- Param node identity: Step-5b's param Def is pinned to the function START line
  (grounding: data_flow.rs registers param defs at fn start) — the gate matches on the
  param node's OWNING FUNCTION identity, never on line arithmetic.
- Lambdas/nested functions in Python get their own function identity (P9 lesson) — a
  descent into a lambda's param would require an Exact CallSite for the lambda; absent
  that it stays a boundary (fine; note it).
- Multi-root traces: depth and parents are per-root; a node reachable at depth 2 from
  root A and depth 0 from root B must not have A's depth block B's expansion —
  **depth must be keyed per root** (like frontier_by_root/parents_by_root). First-
  enqueue-wins parentage is per root already.
- Frontier mode has no sinks but still descends — `frontier_count` grows; the
  fixture's `frontier_count_min` is a floor, safe.
- `enqueued` dedup: a param node reachable BOTH intra-function (same-fn call… not
  possible for a Def param unless recursion) and via descent — first-enqueue-wins is
  acceptable; note any observable ordering effect in the report.
- Windows/encoding: fixture files ASCII, 1-indexed lines.

## 8. Escalation triggers (stop work, write up, hand back to controller)

- Any need to modify Step-5b/build.rs, data_flow.rs, cache versions, or Reachability.
- The gate cannot uniquely recover a CallSite for the flip fixture's shape (would mean
  the grounding is wrong — do not improvise a looser match).
- Any P10 fixture or nav_compat golden fails and the fix isn't obviously in YOUR new
  code.
- Perf: taint_reaches on a real corpus seed regresses >5× wall-clock.
- The sanitizer-walk signature change fans out wider than taint_reaches.rs (unexpected
  consumers).
- Anything that makes you want to add a config flag, a new warning kind, or an
  Evidence field beyond `descent_depth`.

## 9. Execution stages (worktree `p14-taint-descent`, base = current main)

- **Stage A (core)** [M3 re-cut]: S1 + S2 + S3, PLUS the `Relation::CallDescent`
  variant itself with every exhaustive-match/consumer update (the enum lands here so
  Stage A compiles standalone and descent parentage is never mislabeled `DataFlow`),
  fixtures (§3), and T1-T6 + T4a/T4c/T4d/T4e + T9 + T11. Sanitizer threading, witness
  edge-kind string, `descent_depth`, and MCP surface are NOT Stage A — but Stage A
  must leave `sanitized_hits_on_chain` sound in the interim: until Stage B threads
  relations, descent windows must not be evaluated as transitions (acceptable interim:
  a conservative guard skipping cross-function windows by function-identity check,
  replaced by relation threading in Stage B — state which you did in the report).
  Implementer: **openai gpt-5.3-codex-spark via a2a-bridge** (workspace-write; DO NOT
  git commit — controller commits; write `p14-stageA-report.md`).
- **Stage B (witness/evidence surface)**: S4 (sanitizer relation threading replacing
  the interim guard, witness edge kind `"CallDescent"`) + S5 (`descent_depth`,
  counters) + T4b, T7, T8, T10 + acceptance probes. Implementer: **Sonnet 5 (medium
  effort)** subagent, TDD, small commits allowed (trailer
  `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`); write
  `p14-stageB-report.md`.
- Stage B starts only after controller verifies Stage A (build + T1-T6 + fixtures +
  live flip probe). Task reviews (opus 4.8 or gpt-5.5) at controller's discretion per
  stage; whole-branch adversarial review (gpt-5.5 xhigh + controller with empirical
  reproductions) before PR regardless.
- Report format: per-section what changed (file:symbol), tests added with
  failing-first evidence, gate outputs verbatim, deviations + why, ≤15-line summary.

## 10. Global constraints (binding)

- BTreeMap/BTreeSet for all new state; deterministic iteration; 1-indexed lines;
  files <600 lines (split modules if trace.rs would exceed — it is 735 lines already:
  pre-existing, do not grow it by more than the walk refactor requires; extracting the
  shared walk core into `src/cpg/trace_descent.rs` or similar is acceptable).
- ONE shared boundary/descent decision implementation consulted by BOTH walk copies
  (doctrine-6; the current duplication is a known trap).
- Nothing below Exact feeds an asserted finding (consumer-visibility doctrine).
- Additive-only Evidence; existing outputs byte-identical.
