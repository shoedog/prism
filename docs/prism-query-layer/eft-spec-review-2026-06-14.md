# EFT spec — dual spec-review (codex rigor + claude opus soundness) — 2026-06-14

Spec rev 1 (`docs/superpowers/specs/2026-06-14-prism-exact-functionid-traversal-design.md`)
→ **rev 2.** codex gpt-5.5 xhigh (rigor lens, via a2a-bridge `spec-review-eft`, prism MCP;
LSP not exposed) + claude opus 4.8 (soundness lens, operator subagent, prism MCP). Both
verified claims against code. **Both verdicts: tighten** (B1+A core sound; Phase-IP seam
confirmed clean). Raw: `/tmp/eft-spec-review-codex.md`, opus task af55652.

## Findings → disposition (folded into rev 2)

| # | Finding (reviewer) | Disposition (rev 2) |
|---|---|---|
| F1 | **`function_node` re-collapses identity** — returns first same-name candidate, ignoring start_line (codex BLOCKER, query.rs:20) | **Fold §3/§5:** add `function_node_for_id(&FunctionId)` keyed `(file,name,start_line)`; ALL migrations + node-traversal seeds use it (not `function_node`). |
| F2 | **Node API not buildable** — no max_depth, returns no depth; vertical(10)/spiral use depth (codex BLOCKER) | **Fold §3:** `callers_of_node(node, max_depth, ConfidenceFilter) -> Vec<(NodeIndex, usize)>` (BFS, depth, dedup result set). |
| F3 | **Harness wiring wrong** — pin uses `sut.callers()` + `--location` (not `n_by_symbol`/`callers_by_symbol`); no exact-pass evaluator state (codex BLOCKER + opus MAJOR) | **Fold §4/§9:** add `--confidence` to `sut.callers`; keep pin headline on DEFAULT (see F4); add exact P=R=1.0 as a SUPPLEMENTARY assertion (new evaluator handling), not a new `expected` that retires the pin. |
| F4 | **Re-bless honesty** — flipping pin headline to `--confidence exact` masks the default 19 FPs the pin detects; contradicts the adjudication doc (opus MAJOR) | **Fold §1/§9:** pin's PRIMARY outcome stays on default (R=1.0/P=0.21/Phase-IP-pending, per the flip-adjudication doc); `exact` P=R=1.0 is reported as an explicit supplementary metric. |
| F5 | **§2↔§4 "one source of truth" false** — nav resolves on `cg.call_graph` CallSites, never reads CPG Call/Return edges (opus MAJOR, queries.rs:249-441) | **Fold §1/§2/§4:** confidence is materialized in TWO independent places — CPG edges (for §3 node-traversal + §5 slices) and `NavCallEdge.confidence` (for §4 nav). §4 is independent of §2; drop the "one source of truth" framing. |
| F6 | **CHA edges Exact-laundering** — Step-9 guard `matches!(_, Call)` becomes `Call(_)`, so a pre-existing `Call(NameOnly)` pair blocks the Exact CHA add → stays NameOnly (both, build.rs:539/561) | **Fold §2:** Step-9 guard matches `Call(Exact)` (so a NameOnly pair is upgraded by adding the Exact CHA edge), OR promote the existing edge's weight to Exact. Test: NameOnly seed never yields an Exact CHA edge incorrectly. |
| F7 | **Slice call-site-line recovery stays name-keyed** — barrier/membrane/echo re-scan `call_graph.callers.get(func_name)` for the site line after traversal; narrowing the caller SET to Exact but not the line lookup → halves disagree (both, MAJOR) | **Fold §5:** add an exact call-edge helper returning (caller, callee, confidence, call-site line); the migrated slices filter the site-line lookup to the Exact caller set (by caller start_line + resolved target). |
| F8 | **membrane/echo recall-regression** — membrane relies on the R6 single-owner NameOnly demotion for C struct-callback callers (opus, resolution.rs:430) | **Fold §5:** membrane + echo default to `All` (recall-biased, documented); only barrier/vertical/threed/spiral default `ExactOnly`. Per-slice classification table. |
| F9 | **Multi-hop nav `--confidence`** — does `exact` prune the traversal FRONTIER or only emit? nav enqueues after emit (codex MAJOR, queries.rs:282/459) | **Fold §4:** `exact` filters BOTH emission AND frontier expansion; define unresolved-callee handling in exact mode. |
| F10 | **`ResolutionConfidence` lacks serde derives** (both, resolution.rs:8) | **Fold §2:** add `serde::Serialize, Deserialize` to `ResolutionConfidence` as an explicit v6-cache step. |
| F11 | **§8 dedup row misleading** — Call edges are NOT deduped (parallel mixed-confidence A→B possible); only DFG arg edges dedup (opus) | **Fold §8:** scope the dedup note to DFG args; note Call edges can be parallel mixed-confidence (benign for ExactOnly; result-set dedup in F2 handles it). |
| F12 | **Acceptance too thin** for 6 migrations + the policy (codex MINOR) | **Fold §9:** a depth-traversal exactness fixture + a witness-line fixture + the explicit per-slice Exact/All table. |
| F13 | **#12 production path** — name `data_flow.rs` line_start_byte (both ends), not only the witness projection (codex) | **Fold §7:** pin the production line-collapsed case in data_flow, plus the witness projection. |
| F14 | **module_deps is a 2nd nav confidence surface** (codex note, uses NavCallEdge.confidence) | **Fold §10:** note module-deps keeps recall (default `all`); a confidence filter there is deferred with nav. |

**Confirmed sound (no change):** the Phase-IP deferral (default-nav external-receiver
precision via type-confirmed resolution at resolution.rs:416) is a clean additive seam —
re-classifying NameOnly→Exact/dropped before materialization touches no consumer signature,
and fixes BOTH the CPG-edge and nav paths. The ~24 Call/Return blast-radius sites are
accurate (all `matches!`-style; circular_slice/gradient_slice stay confidence-agnostic,
correctly NOT migrated). Cache v6 round-trip is sound (CpgEdge in `Vec<(u32,u32,CpgEdge)>`).

Nothing requires redesign. rev 2 folds F1-F14; next is the owner spec-review gate, then
writing-plans.
