# Exact-`FunctionId` / Confidence-Aware Call Traversal — Design

**Date:** 2026-06-14 · **Status:** rev 2 — dual spec-review folded (codex gpt-5.5 xhigh
rigor + claude opus 4.8 soundness; both "tighten", core sound, Phase-IP seam confirmed
clean). Record: `docs/prism-query-layer/eft-spec-review-2026-06-14.md` (F1–F14). Builds on
**S2 node-identity** (merged `dd60ed6`). Owner forks: **(B1)** tag CPG `Call`/`Return` with
`ResolutionConfidence`, keep both Exact+NameOnly; **(A)** "Exact-path-and-measure" — exact
node traversal + a nav confidence filter; external-receiver NameOnly FPs → **Phase-IP**.

## 0. Why now

S2 de-conflation won *recall*: `fn target` (`TaintSeed::target`, taint.rs:1276) is a distinct
node, so its 5 real callers resolve (`target-c-method` recall 0→1.0). Precision is still
by-name: the nav caller path + the slice queries union every same-named symbol, so the probe
shows raw P=0.21 — 19 FPs, all petgraph `EdgeRef::target()` (S3 R6 single-owner NameOnly
fallback, resolution.rs:416). The CPG **materializes both Exact and NameOnly resolved calls
as `Call`/`Return` edges** (build.rs Step 5), so the false edges are already in the graph.
The fix is **confidence on the edge + exact, confidence-filtered traversal**.

**Two independent confidence surfaces (F5 — NOT "one source of truth"):**
1. **CPG `Call`/`Return` edges** — consumed by §3 node-traversal + §5 slices.
2. **`NavCallEdge.confidence`** (call_resolve.rs:11) — the nav `callers`/`callees` path
   resolves directly on `cg.call_graph` CallSites and **never reads CPG edges**; §4's
   `--confidence` filter operates here, independently of §2.

Both derive confidence from the SAME `resolve_call_site` (resolution.rs:546, `ResolvedCallee.
confidence`), so they agree, but they are wired separately. The Phase-IP fix (re-classify
NameOnly at resolution.rs:416) fixes BOTH.

## 1. Decisions (owner-approved; dual-review folded)

| Decision | Choice |
|---|---|
| Edge confidence (B1) | `CpgEdge::Call(ResolutionConfidence)` + `Return(ResolutionConfidence)`, set from `resolved.confidence` at build.rs Step 5; keep both Exact+NameOnly. |
| Precision target (A) | Exact node traversal (§3) + nav `--confidence` (§4); migrate precision-biased slices to Exact (§5). |
| `target-c-method` re-bless (F4) | **Pin headline stays on the DEFAULT measurement** — record the genuine partial win **R=1.0, P=0.21, Phase-IP-pending** (matching `target-c-method-flip-adjudication-2026-06-14.md`). The Exact-confidence **P=R=1.0** is reported as an explicit **supplementary** metric, NOT a new `expected` that retires the pin. |
| byte-aware arg binding (S2 #9) | IN scope (§6). |
| Low-priority folds | #10 + #12 IN (§7); #11 (CallSite `receiver_recovery`) DEFERRED. |
| Nav default | `--confidence all` (today's recall behavior, byte-for-byte); `exact` opt-in. module-deps keeps recall (F14). |
| Packaging | One `CACHE_VERSION` bump v5→v6 (CpgEdge layout changes). |

## 2. Confidence-tagged edges (`src/cpg/types.rs`, `src/resolution.rs`, `src/cpg/build.rs`)

```rust
pub enum CpgEdge { DataFlow, ControlFlow,
    Call(ResolutionConfidence), Return(ResolutionConfidence), Contains, FieldOf }
```

- **`ResolutionConfidence` gains `serde::Serialize, Deserialize` (F10)** (resolution.rs:8 —
  it lacks them today; required for the v6 cache round-trip). `cpg::types` imports it.
- **Step 5 materialization** (build.rs:359-371): `resolved.confidence` is in hand
  (`ResolvedCallee`, resolution.rs:56-59) → `CpgEdge::Call(resolved.confidence)` +
  `Return(resolved.confidence)`. **No edge dedup today (F11):** A→B can carry two parallel
  `Call` edges at different confidences (one Exact, one R6 NameOnly) — benign (ExactOnly keeps
  the Exact; §3 dedups its result set).
- **Step 9 CHA guard (F6):** the "add virtual edge if none exists" guard (build.rs:561)
  becomes `matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact))` — so a pre-existing
  `Call(NameOnly)` pair is **upgraded** by adding the Exact CHA edge (CHA dispatch is
  type-confirmed = Exact). Without this, a CHA-confirmed call laundered behind an R6 NameOnly
  edge would be wrongly excluded from ExactOnly. Test pins it (§9).
- **Blast radius (~24 sites, all `matches!`-style filters):** circular_slice, gradient_slice,
  build.rs, query.rs (SCC/reachable), navigation/queries.rs (label), cpg/tests.rs, and
  `CpgEdge::is_interprocedural()` — each `…Call | …Return` → `…Call(_) | …Return(_)`.
  circular_slice + gradient_slice stay **confidence-agnostic** (correctly NOT migrated).
- **Cache v5→v6:** `CpgEdge` rides `SerializedCpg.edges: Vec<(u32,u32,CpgEdge)>`; reconstruct
  preserves order; v5 invalidates. PartialHit path unaffected (rebuilds via assemble_graph).

## 3. Node-seeded exact traversal (`src/cpg/query.rs`)

```rust
pub enum ConfidenceFilter { ExactOnly, All }
/// Exact identity lookup — keyed (file,name,start_line), NOT first-candidate (F1).
pub fn function_node_for_id(&self, id: &FunctionId) -> Option<NodeIndex>;
/// BFS over reverse Call edges (== forward Return), filtered by confidence; returns
/// (caller node, depth), deduped by node (F2). Mirrors callers_of's max_depth contract.
pub fn callers_of_node(&self, callee: NodeIndex, max_depth: usize, f: ConfidenceFilter) -> Vec<(NodeIndex, usize)>;
pub fn callees_of_node(&self, caller: NodeIndex, max_depth: usize, f: ConfidenceFilter) -> Vec<(NodeIndex, usize)>;
```

- **F1 (critical):** seeds resolve via `function_node_for_id(&FunctionId)` (start_line-keyed),
  NOT `function_node(file,name)` (which returns the first same-name candidate, query.rs:20, and
  would re-collapse the very identity S2 established).
- **F2:** `max_depth` + `(node, depth)` return (consumers use depth — vertical depth-10,
  spiral); BFS with a `visited` set (like callers_of, query.rs:409) → deduped despite parallel
  mixed-confidence edges. `ExactOnly` skips `Call(NameOnly)` edges at BOTH emit and frontier
  expansion (a NameOnly edge is not traversed in ExactOnly).
- The by-name `callers_of`/`callees_of`/`callers_of_in_file` stay (recall/nav) — additive.

## 4. Nav confidence filter (`src/main.rs`, `src/navigation/queries.rs`, `src/navigation/call_resolve.rs`)

- `prism nav callers|callees` gain `--confidence <exact|all>` (default `all` = today). The nav
  path resolves on `cg.call_graph` + `NavCallEdge.confidence` (INDEPENDENT of §2 CPG edges).
- **Multi-hop (F9):** `exact` filters BOTH the emitted set AND the BFS frontier (nav enqueues
  after emit, queries.rs:282/459) — a NameOnly edge is neither emitted nor expanded in exact
  mode. Document unresolved-callee handling (kept only in `all`).
- **No default change:** `all` output is byte-for-byte today's; the new `--confidence` clap flag
  defaults to `all` (the §9 test asserts unchanged output when the flag is absent).
- **module-deps (F14):** keeps recall (`all`); a confidence filter there is deferred with nav.
- **Tier-A harness:** thread `--confidence` through `sut.callers` (sut.py:206); the pinned
  probe (pinned.py:110, location-seeded `sut.callers` + `--location`) reports BOTH default
  (headline) and exact (supplementary) — see §9.

## 5. Slice migration (`src/algorithms/`) — per-slice Exact/All (F7, F8)

Seed via `function_node_for_id` → `callers_of_node`/`callees_of_node`. Per-slice confidence:

| Slice | Filter | Why |
|---|---|---|
| barrier, vertical, threed, spiral | **ExactOnly** | precision-biased; same-name over-report is the bug. |
| membrane, echo | **All** | recall-biased — membrane relies on the R6 single-owner NameOnly demotion for C struct-callback callers (resolution.rs:430); ExactOnly would DROP them (regression). |

- **Call-site-line consistency (F7):** barrier/membrane/echo recover the call line via
  `call_graph.callers.get(func_name)` after traversal. Add an **exact call-edge helper**
  returning `(caller_id, callee_id, confidence, call_site_line)`; the migrated slices take
  their site lines from it, filtered to the same caller set (by caller `start_line` + resolved
  target) — so the caller set and the site lines agree. (For `All` slices, behavior is
  today's.)

## 6. byte-aware interprocedural arg binding (S2 #9)

Step-5b binds args via `call_argument_texts(site.line, callee)` (ast.rs:4197), stopping at the
first same-line call. Add `call_argument_texts_at(start_byte, callee)` selecting the call expr
by `site.start_byte` (now on `CallSite`); Step-5b uses it so `callee(a); callee(b)` each bind
their own args. Existing line-keyed method retained for other callers.

## 7. Folded low-priority S2 deferrals

- **#10** (`collect_identifier_path_spans`, ast.rs:2130): `o.config.timeout += 1` peels nested
  receivers to the leftmost identifier before the base `Use(o)` fallback.
- **#12 (F13):** pin the line-collapsed anchor in the **production path** — `data_flow.rs` uses
  `line_start_byte` for both ends of a line-collapsed reference (assert `start==end` there),
  plus the witness-layer projection.
- **#11 deferred** (CallSite `receiver_recovery` in `cmp_key`).

## 8. Failure modes

| Mode | Behavior |
|---|---|
| `callers_of_node(ExactOnly)` with only NameOnly callers | Empty (precision over recall, by design); recall view is `All` / nav default. |
| Parallel mixed-confidence A→B Call edges (F11) | Both exist (Step 5 no dedup); ExactOnly keeps the Exact; result-set dedup (§3) collapses A. DFG *arg* edges still dedup as before — the only dedup'd class. |
| Confidence-agnostic consumers (circular/gradient, unmigrated) | Unaffected — match `Call(_)`/`Return(_)`. |
| CHA edge whose pair already has a NameOnly Call edge | Upgraded to Exact via the §2 guard (F6). |
| Cache v5 read by v6 | Rejected by `CACHE_VERSION`. |

## 9. Testing & acceptance

1. **Edge confidence** (`tests/ast/cpg_test.rs`): a known Exact call → `Call(Exact)`; an R6
   single-owner call → `Call(NameOnly)`.
2. **CHA upgrade (F6):** a virtual-dispatch pair that also has an R6 NameOnly Call edge ends up
   with an `Exact` edge (not laundered).
3. **`function_node_for_id` (F1):** two same-name functions → distinct nodes; `for_id` returns
   the start_line-matched one (not first-candidate).
4. **`callers_of_node` filter + depth (F2):** for `fn target`'s node, `ExactOnly` returns the
   in-repo callers (excludes the petgraph `.target()` NameOnly sites); `All` includes them;
   `max_depth`/`(node,depth)` honored; result deduped.
5. **Slice precision + per-slice policy (F12):** a barrier/vertical fixture (two same-name fns →
   no cross-over-report, incl. consistent call-site LINES via the F7 helper); a membrane/echo
   fixture proving `All` still surfaces the R6 C struct-callback caller (no recall regression).
6. **byte-aware arg binding (#9):** `callee(a); callee(b)` on one line → both `a`→param and
   `b`→param edges.
7. **#10 / #12:** nested-augmented base `Use(o)`; data_flow line-collapsed `start==end`.
8. **Tier-A re-bless (F3/F4):** `sut.callers` gains `--confidence`; the `target-c-method` pin's
   **headline outcome = default** (R=1.0, P=0.21, recorded as the partial win / Phase-IP-pending,
   matching the adjudication doc); add a **supplementary** assertion that exact-confidence
   callers give P=R=1.0 (new harness wiring — NOT a new `expected` that greens-and-retires the
   pin; the evaluator records exact as a separate metric).
9. **Nav default unchanged:** nav output with `--confidence` absent == today (byte-for-byte).
10. **Cache v6 round-trip** + v5-invalidates (CpgEdge confidence survives reconstruct).
11. **Repo Tier-A workflow** (CLAUDE.md): `cargo build --release` then `uv run tier-a
    --matrix-only --allow-stale-sut` (exit 0) then `--quick`; full `cargo test` (+ `--features mcp`).

## 10. Out of scope / deferred

- **Phase-IP — default-nav external-receiver precision** (R6 single-owner attributing
  `.target()`-on-`petgraph::EdgeRef` to the in-repo method). Needs type-confirmed receiver
  resolution (S3.1 / Phase-IP). *Seam (confirmed clean by both reviews):* re-classify
  NameOnly→Exact/dropped at resolution.rs:416 **before** materialization — changes only *which*
  edges/sites exist, no consumer signature; fixes BOTH the CPG-edge and nav paths (so the
  DEFAULT `target-c-method` P→1.0 lands here). This is the partial win's completion.
- **module-deps confidence filter** (F14) — deferred with nav; keeps recall now.
- **S2 #11** (CallSite `receiver_recovery` Ord/Eq).
- **Plan B (`taint_reaches`)** — separate; this increment is a natural precursor (confidence-
  tagged edges + exact traversal aid a witness consumer).
