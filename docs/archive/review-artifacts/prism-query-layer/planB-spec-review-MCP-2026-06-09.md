Both lenses (Rigor = completeness/ambiguity; Soundness = design/decomposition) returned full reviews — no node failed, so the synthesis below draws on both. They converge on the same factual findings; the main divergences are (a) severity of the data-acquisition issue and (b) whether the wire-size chokepoint is a real gap. Both are resolved inline.

---

## BLOCKER

**1 — §1 dogfood + §4 step 2 — The per-query CPG rebuild rests on a false premise and names APIs that don't fit.**
Both lenses independently verified that the spec's claim — *"`from_ctx` keeps only the `CodePropertyGraph` and discards the DFG"* — is wrong: the DFG lives **inside** `CodePropertyGraph` (`pub dfg`, `build.rs:47-49`), `from_ctx` retains `ctx.cpg` wholesale (`navigation/mod.rs:67`), and the persisted cache round-trips the full graph + all DataFlow/ControlFlow edges + `dfg` (`cpg_cache.rs:78-84,322-340`). The named mechanism also doesn't fit: `build_scoped` is diff-driven and silently falls back to a full whole-repo build at >50% of files or ≤1 file (`context.rs:135-158`); `build_subset` returns a bare `DataFlowGraph` with no CFG edges and no `taint_forward_cfg`. This also contradicts Plan A's "reasoning overlay over CPG node indices" seam.
*Disagreement resolved:* Rigor rates this BLOCKER, Soundness MAJOR. **Rigor is right** — as written, §4 step 2 is literally unimplementable with the cited APIs (an implementer must invent subset-CPG plumbing or hit a per-query full-rebuild latency cliff), so it blocks planning. Soundness's steelman still holds and should be folded in: for intraprocedural-v1 the *answer* (frontier/reached) is **scope-invariant**, so only cost and `InterproceduralBoundary` warning counts vary.
→ **Resolution:** Query `index.cpg.taint_forward_cfg(&seeds)` directly (zero rebuild, Option-C intact). Delete the "discards the DFG" premise and the `build_scoped`/`build_subset` references. If profiling later shows the interprocedural DFG fan-out is too large, bound the traversal *in-engine* (the engine is already writing a custom `dfg_witness_path` BFS) rather than rebuilding a `CpgContext` per query.

**2 — §4/§7/§9 — The tool's primary answer (`reached`) is not machine-readable as written.**
`Evidence` has no boolean/summary field (`navigation/types.rs:118-127`) and `GraphNode` has no `why` (`:100`); in witness mode `items` is empty (one-shape rule), so the entire yes/no answer would ride in a `Warning`'s kind+message string. Rigor flags it as unimplementable; Soundness adds the durable cost — §9 makes this shaping the template for three more Tier-2 tools, so every future tool inherits a prose-encoded primary answer.
*Disagreement resolved:* Rigor BLOCKER vs Soundness MAJOR — **treat as BLOCKER** because the tool's whole output is a boolean and four tools copy the shape, but note it has a clean, byte-safe fix.
→ **Resolution:** Add an additive top-level `Option<ReasoningSummary { reached, counts }>` to `Evidence`, guarded by `#[serde(skip_serializing_if = "Option::is_none")]` — byte-safe and **proven** by the existing `Evidence.graph` field, so nav goldens stay byte-identical. Specify per-sink results explicitly for the **mixed reached/unreached** case. Decide this seam now, before §9 generalizes it.

**3 — §3 vs §7 — Error/empty-result semantics conflict (Rigor-only; Soundness did not surface this).**
§3 says per-seed failures degrade to warning+skip but all-empty → `QueryError`; §7 says no-sources-resolved / source-outside-function → empty `Evidence` + warning (not an error).
→ **Resolution:** Add a truth table covering: mixed per-seed failures, *all* source failures, outside-function locations, zero-`Variable` seed lines, unresolved sinks, and *all* sink failures — and which produce `QueryError` vs empty-Evidence+warning.

**4 — §3/§4 — Seed and sink matching underspecified.**
Rigor: the spec defines `SeedSpec` as `Loc|Symbol` (`:67-77`) but not the `FocusSet`/`SeedSet` wire shape, `ResolvedSeed` fields, whether resolution returns `(file,line)` or exact `VarLocation`s, or sink membership semantics. A line can hold multiple `Variable` nodes (current taint starts from *every* `Variable` at a line, `cfg_queries.rs:149-164`); function-symbol seeds should taint **params only**, not every variable at the start line.
→ **Resolution:** Define the `FocusSet`/`SeedSet` JSON shape and `ResolvedSeed` fields; state resolution granularity (`(file,line)` vs `VarLocation`); specify function-symbol = params-only; and state whether sink membership is any / all / a specific variable.

---

## MAJOR

**5 — §4 vs §7 — Witness response shape contradicts unreachable-sink behavior.** §4 says witness mode is graph-only (never both items and graph); §7 says unreachable resolved sinks return `reached:false` "with the frontier as evidence." → Pick one: graph-only with a summary-warning, or item-frontier fallback. Pin it in tests.

**6 — §4 vs §8 — Edge-kind vocabulary contradicts itself, and the witness traversal must match the frontier's edge relation.** Rigor: §4 requires `kind:"DataFlow"` + `"AssignmentPropagation"`; §8 expects `kind:"TaintFlow"`. Soundness adds a load-bearing invariant: `dfg_forward_reachable` reaches frontier members via DataFlow edges **plus same-line assignment propagation** (`query.rs:541-559`), so the new `dfg_witness_path` (A3) must use the *identical* edge relation or it can fail to reconstruct a path for a sink the membership test already confirmed reachable. → Choose the exact edge-kind strings, and make "witness edge relation ≡ frontier edge relation" an explicit, tested invariant.

**7 — §4/§5 — Warning/Reason vocabulary needs a typed contract, and the boundary warning must bind to the sink.** Rigor: `WarningKind` is a closed 7-variant enum (`navigation/types.rs:80-88`), so every new warning and `Reason::TaintedBy` requires explicit variant additions; resolve `InterproceduralBoundary` vs `ReachesFunctionBoundary`, define `CleansedFlow`, the summary warning, dedup/counts, and message format. Soundness adds: an unbound `InterproceduralBoundary` warning leaves an agent unable to tell a *deferral* `reached:false` from a *true-negative* `reached:false`, especially with multiple sinks. → Specify exact `Reason::TaintedBy` fields and `WarningKind` variants; emit a **sink-located** boundary warning when a resolved sink lies outside the source function.

**8 — §4 + Plan A dep — Sanitizer plumbing needs a Plan B-facing API.** Both lenses confirmed the public CFG/CPG taint helpers hardcode empty `cleansed_for` (`cfg_queries.rs:198`, `query.rs:617-626`); the real cleanser is private `apply_cleansers`, called only inside `taint::slice` (`taint.rs`). Without a named callable seam, an implementation can legally return permanently empty `cleansed_for`. → Name the `pub(crate)` seam Plan B calls and state when cleansing is applied (currently gated to Go/Python/JS-TS).

**9 — §4 — Witness determinism is incomplete.** Plan A locks "Shortest path v1," but Plan B doesn't import the multiplicity/tie-break contract. → Specify source/sink ordering, neighbor ordering, cycle handling, multiple-path graph composition, and `max_results` interaction.

**10 — §5 — MCP surface incomplete.** Clarify the library name `taint_reaches` vs the MCP tool `reason_taint_reaches`, then provide the full hand-authored JSON schema for `sources`, optional `sinks`, `max_results`, and `verbosity`, matching the existing parser/schema style.

**11 — §6 — Wire-size chokepoint behavior underspecified (the two lenses partly disagree).** *Disagreement resolved:* Soundness verified that `write_message` **is** a sound single chokepoint — both success and error frames already route through it (`transport.rs:68`), and the only other impl (`:462`) is `#[cfg(test)] InMemoryTransport`, never a real wire — so Rigor's implicit "second unbounded path" worry is unfounded. **But Rigor is right that the chokepoint's behavior is unspecified:** `write_message` today only serializes (`transport.rs:428-432`), error `_meta` lacks `anthropic/maxResultSizeChars` (`error.rs:162-168`), and JSON-RPC protocol errors have no `_meta` path (`transport.rs:295-304`). → Keep the chokepoint placement (it's correct); specify what it does when a frame exceeds the cap (replace? truncate?), how protocol errors carry cap metadata, and the valid-JSON guarantee for success / tool `isError` / terminal over-cap / protocol-error responses.

---

## MINOR

**12 — §3 vs §5 — Naming drift in the shared seam (both lenses).** Resolved set is `FocusSet` (§1/2/4/5/8) vs `SeedSet` (§3); module is `focus.rs` (§5/8) vs `seeds.rs` (§3); input element is `SeedSpec`. Since all four Tier-2 tools build on this seam, pick one name for the resolved set, one for the input element, and one module path before writing.

**13 — §2/§4/§5 — Receiver mis-attribution (Soundness-only).** `taint_forward_cfg` exists only on `CodePropertyGraph` (`cfg_queries.rs:129`), called as `ctx.cpg.taint_forward_cfg` (`taint.rs:10839`); the spec writes `CpgContext::taint_forward_cfg` and `cpg::CpgContext::{build_scoped, taint_forward_cfg}`, conflating two receivers (`build_scoped` *is* on `CpgContext`; `taint_forward_cfg` is not). → Align so an implementer doesn't hunt on the wrong type.

**14 — §6 — Split the `default-run` fix into its own commit (Soundness-only).** `default-run = "prism"` is a pure CLI-ergonomics fix bundled into a transport/error-bounding deliverable; land it separately so a transport hiccup can't block it.

**15 — §4 citation nit (both lenses).** The additive catch-all arm is at `src/output/navigation.rs:73`, not `:42` (line 42 is the `Reason::Calls` arm). The substantive point — new `Reason`/`WarningKind` variants serialize/render safely — is correct.

**Not a defect (confirmed by Soundness):** Plan A gating is honest — `cleansed_for` genuinely can't populate until A4, and the witness genuinely needs A3's parent-tracking; these dependencies are stated accurately. Placement (`src/reasoning/` overlay), Option-C non-modification, intraprocedural-v1 + visible boundary, and one-shape-per-response are well-verified and sound.

---

**Verdict: Not ready to plan — resolve the four BLOCKERs first.** Specifically: (1) correct the data-acquisition path to query the already-retained `index.cpg.taint_forward_cfg` and drop the false "discards DFG" premise; (2) make `reached` machine-readable via an additive `skip_serializing_if`-guarded `Evidence` summary before the shape propagates to three more tools; (3) add the error/empty-result truth table; (4) pin the `FocusSet`/seed/sink wire shape and matching semantics. The MAJORs (witness shape, edge/warning vocabulary, sanitizer seam, determinism, MCP schema, wire-cap behavior) should be settled in the same revision but don't require redesign.