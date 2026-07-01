# Phase-IP Foundation (rev 4) — Round-4 SOUNDNESS review (claude opus, operator-subagent)

> Run as an operator-driven subagent (the a2a-bridge claude reviewer leg is the open `{{reviewer_claude}}`
> model-override defect). Complements the codex rigor review (`phase-ip-spec-review4-codex-2026-06-15.md`).
> Read-only verification against the post-`e03f547` (embedding-merged) tree.

I verified every load-bearing citation against the post-`e03f547` tree. The architecture (sibling-to-embedding
apply/clear, the `resolve_call_site` seam at resolution.rs:438, FunctionId minting from `GoMethod`, the
harness apparatus seams) is real and correctly cited. The split is **not a fiction** — PR-1's engine is
genuinely buildable on receivers P6-lite already types, and PR-2's receiver expansion feeds the same
`interface_impls` with no engine rework. But two findings undercut the spec's stated PR-1 *value/validation*
story, and both are about the same blind spot: the empty-live fallback.

## BLOCKER-1 — §13.1 "fallback Exact is load-bearing for the capability flip" is FALSE; the matrix flip can't see confidence
**Spec:** §0/§13.1(a), §14(e). **Code:** matrix.py:74-77; sut.py:78-89; main.rs:353; queries.rs:272,296.

The matrix calls `sut.callers` → plain `prism nav callers` (no `--confidence`, so `exact_only=false`,
queries.rs:272) → emits **both** Exact and NameOnly edges (the filter at queries.rs:296 only fires when
`exact_only=true`). The matrix compares **call-site locations only** (`got == case.expect_callers`,
matrix.py:76); `case.exact=true` selects set-equality-vs-subset, **not** a confidence assertion. So
`go/interface_dispatch` flips `known_fail → ok` whether the fallback mints Exact **or** NameOnly. The
fallback's Exact-ness only matters in **ExactOnly** traversal (barrier/vertical/threed, barrier_slice.rs:107),
which the capability fixture never exercises. The decision to keep the fallback Exact may still be right
(owner-locked), but the *justification in §13.1* is wrong and will mislead the planner.

**Resolution:** Correct §13.1 — the flip is confidence-blind (default nav + location-only matrix); the
fallback's Exact-ness is validated **only** by an ExactOnly consumer. Add a PR-1 test driving the empty-live
fallback through an ExactOnly slice (or `nav callers --confidence exact`) on a constructs-nothing fixture,
asserting the edge **survives** the Exact filter.

## BLOCKER-2 — the most precision-risky path (empty-live fallback → wide Exact fan-out into ExactOnly) has NO PR-1 test and NO active gate
**Spec:** §9, §12 (row "no admission key … live"), §13.7, §14(d)/(e). **Code:** barrier_slice.rs:90-107; live_types.rs:158-172; §14(e) declares the corpus gate dormant.

The genuinely dangerous case is the empty-live fallback: when no admission key is live, the spec mints the
**full satisfier set** as Exact into precision-biased ExactOnly slices. §13.7 instantiates its implementers →
travels the **live-intersection** path, so it does not exercise the fallback fan-out. §14(e) makes the corpus
precision gate **dormant** in PR-1 (caddy-neutral). Net: the one path that can dump a wide same-method
satisfier set (`error`-class) into barrier/vertical/threed as Exact is exercised by no fixture and no gate
in PR-1 — "ships broken in PR-1, surfaces in PR-2."

**Resolution:** Add a PR-1 fixture for the fallback path under ExactOnly with ≥2 satisfiers and *no*
construction: assert the barrier/DataFlow fan-out is exactly the (full) satisfier set and non-satisfiers do
not leak. This makes §13.7's precision guard cover the fallback, not just the live path.

## MINOR-1 — §11 scoped-build wording incorrect; benign for a stronger reason
**Spec:** §3 (Scoped build), §11 (BLOCKER-4). **Code:** context.rs:160-170; build.rs:127-138; mod.rs:34-40.

`build_scoped` builds the CPG on `filtered` (the scoped subset), so `CallGraph::build(filtered)` computes
interface_impls over the **subset**, not the full repo — and it does not "mirror `live_types`" (a separate
`collect_live_types(files)` over the full set, context.rs:170). Embedding has this exact behavior today. **The
consequence is benign for a stronger reason:** `NavigationIndex::build` always uses `CpgContext::build`
(whole-repo) and never `build_scoped` (mod.rs:34 `debug_assert!(ctx.scope.is_none())`). So nav — the success
metric — always resolves on a full-repo owner index + full-repo interface_impls; scoped mode only affects
best-effort slice edges.

**Resolution:** Reword §11: scoped build computes interface_impls over the scoped subset (same as embedding);
only affects best-effort slice edges; nav never uses `build_scoped` (mod.rs:34), so its owner index +
interface_impls are always full-repo. Drop "mirrors live_types" and "full repo file set."

## MINOR-2 — §8 overstates the scan_go delta (traversal already whole-tree)
**Spec:** §8. **Code:** live_types.rs:154-173 + scan_tree_recursive:275-285.

`scan_go` already runs via `scan_tree_recursive`, which walks the entire AST including factory bodies. So a
factory body's `&Foo{}` is already visited today for the value case. The genuine §8 delta is the
**admission-key alphabet**: emitting `*T` for `&T{}`/`new(T)` and handling `var x T`. Strengthens soundness
(liveness is more real than credited) but the framing could misdirect implementation toward "add body
recursion" rather than "widen `scan_go_node`."

**Resolution:** Reword §8: the existing `scan_go` already recurses whole-tree (incl. factory bodies); the
change is to `scan_go_node` (live_types.rs:158) to emit the `{T, *T}` admission-key alphabet.

## Confirmed SOUND (no action)
- Architecture / one-ladder (§1,§2,§5): seam at resolution.rs:438 is the P6-lite-miss arm; `owner_lookup("Runner","Go")` genuinely misses; `exact()` (resolution.rs:169) takes N callees and composes with Step-5. ✔
- P6-lite premise (Q1): `go_parameter_binds_name` (ast.rs:3968) recovers `r:Runner`; the fixture constructs nothing → empty RTA → fallback path, as §13.1 states. ✔
- Replace-not-merge (§11, Q5): build_incremental mirrors embedding (remove_files→clear at call_graph.rs:768; merge carries no dispatch maps 775-792; build_direct_subset inits empty; post-merge apply over full set). No stale interface_impls survives. ✔
- Harness apparatus seams (§14): all citations accurate (types.rs:61-63, queries.rs:326, sut.py:78-105, model.py:36-41, strata.py:10/27-34, adjudication.py:79-125). Caveat folded into BLOCKER-1/2.
- Step-9 CHA reconciliation (§17): [SUPERSEDED — see codex review + operator adjudication: `virtual_method_nodes` is name-keyed over ALL func_index entries (build.rs:534), so this "sound" call was WRONG; codex's MAJOR stands and is folded.]
- FunctionId minting (§5/§10): the PromotedMethod path (go.rs:630-635) already mints FunctionId from a GoMethod; `compute_interface_dispatch -> Vec<FunctionId>` is constructible identically. ✔

**Verdict: needs changes** — sound to plan PR-1 after fixing BLOCKER-1 (correct the false linchpin rationale +
add an ExactOnly-fallback assertion) and BLOCKER-2 (a fallback-path precision fixture). Both small/additive;
the engine design is sound and the split is real. Fold the two MINORs as spec-accuracy corrections. (Operator
note: the §17 "confirmed sound" item was overturned by the codex review and verified against build.rs:534 —
CHA is language-blind; that MAJOR is folded too.)
