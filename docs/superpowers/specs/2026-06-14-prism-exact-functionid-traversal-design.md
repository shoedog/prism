# Exact-`FunctionId` / Confidence-Aware Call Traversal — Design

**Date:** 2026-06-14 · **Status:** rev 1 (brainstormed; owner-approved the two forks).
Builds on **S2 node-identity** (merged `dd60ed6`). Two owner decisions (brainstorming):
**(B1)** tag CPG `Call`/`Return` edges with `ResolutionConfidence`, keeping both Exact +
NameOnly; **(A)** "Exact-path-and-measure" — deliver exact traversal + a nav confidence
filter and re-bless `target-c-method` to Exact-confidence precision; the NameOnly
external-receiver FPs are a separately-scoped **Phase-IP** (type-confirmed resolution)
concern, NOT this increment. Context: the S2 Tier-A run flipped `target-c-method`
(`docs/eval/tier-a/target-c-method-flip-adjudication-2026-06-14.md`).

## 0. Why now

S2 de-conflation gave prism the *recall* half of correct call resolution: `fn target`
(`TaintSeed::target`, taint.rs:1276) is now a distinct `(file,name,start_line)` node, so its
5 real callers resolve (`target-c-method` recall 0→1.0). But **precision is still by-name**:
`callers_of`/`callees_of`/`callers_of_in_file` and the nav caller path union *every*
same-named symbol, so the probe shows raw P=0.21 — 19 FPs that are all petgraph
`EdgeRef::target()` calls (verified: trace.rs:241, build.rs:542, query.rs:125, … all
`e.target()`). Codex's key finding: a naive node reverse-traversal **won't** fix this —
the CPG **materializes both Exact and NameOnly resolved calls as `Call`/`Return` edges**
(build.rs Step 5), so the false `.target()→TaintSeed::target` edges (S3 R6 single-owner
NameOnly fallback, resolution.rs:416) are already in the graph. The fix is **confidence on
the edge** + **exact, confidence-filtered traversal**.

## 1. Decisions (owner-approved)

| Decision | Choice | Rationale |
|---|---|---|
| Edge confidence | **`CpgEdge::Call(ResolutionConfidence)` + `Return(ResolutionConfidence)`** — tag at materialization, keep BOTH Exact + NameOnly | The CPG becomes confidence-aware = one source of truth; precision-biased consumers filter Exact, recall-biased (nav default) keep NameOnly. (B1, over re-resolve-at-query and over drop-NameOnly.) |
| Precision target | **Exact-path-and-measure** | Add exact node traversal + a nav `--confidence exact` filter; re-bless `target-c-method` to measure prism's EXACT caller precision (P=R=1.0 — honest: the 5 Exact). Nav DEFAULT stays recall-biased (`all`, NameOnly@0.6). |
| `target-c-method` | **Re-bless to Exact-confidence P=R=1.0**; default-nav NameOnly FPs → Phase-IP | The Exact layer IS precise; the recall layer's external-receiver precision is type-confirmed dispatch (deferred). |
| byte-aware arg binding (S2 deferred #9) | **IN scope** | Same "call precision" theme; `CallSite` byte already exists; `call_argument_texts` byte-key is small + completes the S2 de-collapse. |
| Low-priority folds | **#10 (nested-augmented base fallback) + #12 (line-collapsed witness test) IN; #11 (CallSite `receiver_recovery` in `cmp_key`) DEFERRED** | #10/#12 clean + cheap; #11 perturbs the de-collapse dedup key and is pre-existing/not-reachable — not worth touching here. |
| Nav default | **`--confidence all` (today's recall behavior); `exact` is opt-in** | No regression to nav/recall consumers; the harness/probe opt into `exact`. |
| Packaging | One increment, one `CACHE_VERSION` bump v5→v6 | `CpgEdge` is serialized; tagging changes its layout. |

## 2. Confidence-tagged edges (`src/cpg/types.rs`, `src/cpg/build.rs`)

```rust
pub enum CpgEdge {
    DataFlow,
    ControlFlow,
    Call(ResolutionConfidence),    // was unit; now carries Exact|NameOnly
    Return(ResolutionConfidence),  // mirrors the Call it pairs with
    Contains,
    FieldOf,
}
```

- `ResolutionConfidence` (resolution.rs:9) is re-exported / imported by `cpg::types`.
- **Materialization (build.rs Step 5):** `for resolved in cg.resolve_call_site(site)` already
  yields `resolved.confidence` (ResolvedCallee, resolution.rs:56-59). The two `add_edge`
  calls become `CpgEdge::Call(resolved.confidence)` and `CpgEdge::Return(resolved.confidence)`.
  Step 9 virtual-dispatch (CHA) edges are `Exact` (type-confirmed by construction).
- **Blast radius (~24 sites, compiler-enumerated):** every `matches!(e, CpgEdge::Call | CpgEdge::Return)`
  filter (cpg/query.rs SCC/reachable, circular_slice, gradient_slice, build.rs, navigation/
  queries.rs label, cpg/tests.rs) becomes `…Call(_) | …Return(_)`; `CpgEdge::is_interprocedural()`
  (types.rs) matches `Call(_) | Return(_)`. Behavior unchanged for confidence-agnostic
  consumers — they keep matching both.
- **Cache:** `CACHE_VERSION` 5→6; `CpgEdge` round-trips through `reconstruct_cpg` (serialized
  `Vec<(u32,u32,CpgEdge)>`) — the confidence rides for free once the enum derives are intact.

## 3. Node-seeded exact traversal (`src/cpg/query.rs`)

```rust
pub enum ConfidenceFilter { ExactOnly, All }   // ExactOnly = precision; All = recall

/// Callers of a SPECIFIC function node (no name union). Walks reverse `Call` edges
/// (== forward `Return`), filtering by confidence.
pub fn callers_of_node(&self, callee: NodeIndex, filter: ConfidenceFilter) -> Vec<NodeIndex>;
pub fn callees_of_node(&self, caller: NodeIndex, filter: ConfidenceFilter) -> Vec<NodeIndex>;
```

- Seeded by a `NodeIndex` (from `function_node`/`function_candidates`/`function_at`), so it
  resolves callers of *that* overload only — the S2 identity payoff. `ExactOnly` skips
  `Call(NameOnly)` edges (drops the petgraph `.target()` FPs).
- The existing by-name `callers_of`/`callees_of`/`callers_of_in_file` **stay unchanged** (the
  recall/nav surface). New APIs are additive.

## 4. Nav confidence filter (`src/main.rs`, `src/navigation/queries.rs`, `src/navigation/call_resolve.rs`)

- `prism nav callers|callees` gain `--confidence <exact|all>` (default `all` = today). Threaded
  into `navigation::queries::callers/callees`. `NavCallEdge` already carries `confidence`
  (call_resolve.rs:11), so the filter drops `NameOnly` edges at emit when `exact`.
- **No change to default nav output** (byte-for-byte for `all`); `exact` is a new, opt-in mode.
- **Tier-A harness:** `eval/tier_a/sut.py::n_by_symbol` (runs `prism nav callers --symbol …`)
  passes `--confidence exact` for the pinned `target-c-method` probe (and the probe's
  measurement is the Exact set). The supplementary `all` measurement may still be reported.

## 5. Slice migration (`src/algorithms/`)

Migrate the by-name slice consumers to `function_at → node → callers_of_node/callees_of_node`
with `ConfidenceFilter::ExactOnly`, so same-name functions stop over-reporting:
**`vertical_slice`, `threed_slice`, `barrier_slice`, `spiral_slice`, `membrane_slice`,
`echo_slice`** (all call `ctx.cpg.callers_of_in_file`/`callees_of` today). Each: resolve the
seed `FunctionId` → `function_node`/`function_candidates` → node-traversal. Where a slice is
deliberately recall-biased (document per algorithm), it may pass `All` — but default to
`ExactOnly`. Behavior changes are EXPECTED precision gains; pinned by §9 tests + the existing
algorithm suites (expected-flip discipline).

## 6. byte-aware interprocedural arg binding (S2 deferred #9) (`src/ast.rs`, `src/cpg/build.rs`)

Step-5b binds args via `call_argument_texts(site.line, callee_name)` (ast.rs:4197), which stops
at the first same-line call → `callee(a); callee(b)` both bind `a`. Add a byte-keyed sibling
`call_argument_texts_at(start_byte, callee_name)` (selects the call expr by `site.start_byte`,
now on `CallSite`); Step-5b uses it so each duplicate binds its own args. Existing line-keyed
method stays for other callers.

## 7. Folded low-priority S2 deferrals

- **#10 nested-augmented base fallback** (`collect_identifier_path_spans`, ast.rs:2130): for
  `o.config.timeout += 1`, peel nested field/index receivers to the leftmost identifier before
  the base `Use(o)` fallback (today only fires when the immediate receiver is an identifier).
- **#12 line-collapsed witness anchor test**: pin that a line-collapsed use's witness byte is
  `start==end` (the §6/S2 best-effort boundary).
- **#11 deferred** (CallSite `receiver_recovery` Ord/Eq) — out of scope (touches de-collapse
  `cmp_key`; pre-existing, not reachable).

## 8. Failure modes

| Mode | Behavior |
|---|---|
| `callers_of_node(ExactOnly)` on a function with only NameOnly callers | Returns empty (precision over recall — by design); the recall view is `All` / nav default. |
| Confidence-agnostic consumers (most algorithms not migrated) | Unaffected — they match `Call(_)`/`Return(_)`; behavior identical to today. |
| CHA / virtual-dispatch edges | Tagged `Exact` (type-confirmed); included in `ExactOnly`. |
| Cache v5 read by v6 | Rejected by `CACHE_VERSION` (v5 invalidates), as in S2. |
| Same-line dup calls, same args | byte-key binds each; identical args → identical (deduped) edges (harmless). |

## 9. Testing & acceptance

1. **Edge confidence** (`tests/ast/cpg_test.rs`): a known Exact call and a known NameOnly
   (R6 single-owner) call materialize `Call(Exact)` vs `Call(NameOnly)`.
2. **`callers_of_node` filter** (`tests/ast/` / `tests/integration/`): for `fn target`'s node,
   `ExactOnly` returns the in-repo callers and **excludes** the petgraph `.target()` NameOnly
   sites; `All` includes them. The direct unit analogue of the `target-c-method` win.
3. **Slice precision** (`tests/integration/`): two same-name functions; a migrated slice
   (e.g. barrier/vertical) traverses only the seeded overload's callers (no name union).
4. **byte-aware arg binding** (#9): `callee(a); callee(b)` on one line → `a`→param and
   `b`→param edges both present (was only `a`).
5. **#10 / #12**: nested-augmented `o.x.y += 1` emits base `Use(o)`; line-collapsed witness
   `start==end`.
6. **Pinned re-bless** (`eval/tier_a/pinned.py` + `sut.py`): `target-c-method` measured at
   `--confidence exact` → **P=R=1.0** (re-bless `expected` from `known_fail` to the exact-pass
   state). Default `all` still records the NameOnly FPs (Phase-IP target).
7. **Repo Tier-A workflow** (CLAUDE.md): `cargo build --release` then `uv run tier-a
   --matrix-only --allow-stale-sut` (exit 0) then `--quick`; plus full `cargo test`
   (default + `--features mcp`).
8. **Cache**: v6 round-trip + v5-invalidates (`tests/ast/cpg_cache_test.rs` — `CpgEdge`
   confidence survives reconstruct).

## 10. Out of scope / deferred

- **Phase-IP — default-nav precision for external-receiver NameOnly** (the R6 single-owner
  fallback attributing `.target()`-on-`petgraph::EdgeRef` to the in-repo method). Needs
  type-confirmed receiver resolution (the S3.1 / Phase-IP receiver-typing). *Seam:* edges are
  now confidence-tagged; a later pass can re-classify NameOnly→dropped/Exact at the resolution
  layer without touching consumers. This is the default-`all` `target-c-method` precision win.
- **S2 #11** (CallSite `receiver_recovery` Ord/Eq) — see §7.
- **Plan B (`taint_reaches`)** — separate increment; this one is a natural precursor (exact
  caller/callee traversal + confidence-tagged edges are useful to a witness consumer).
