Both lenses returned complete reviews (no node failed), so the synthesis below draws on each: Rigor for completeness/ambiguity gaps that block independent implementation, Soundness for design-correctness. Where they diverge, I resolve it in one line inline.

---

## BLOCKER

**1. §4 (steps 2 & 4) / M6 — the "membership-confirmed sink always reconstructs" invariant is not guaranteed by construction.**
*Issue:* The spec defends M6 by having the witness use the *identical edge relation* as the frontier (DataFlow + same-line propagation). That is true but addresses the wrong axis: frontier = an *unconfined* `dfg_forward_reachable` transitive closure **then** a node-wise CFG-line filter; witness = the *same* BFS **confined to surviving frontier nodes**. For a chain `S→I→K` the filter can keep `K` (line ∈ cfg_set) while dropping its only path-predecessor `I` (line ∉ cfg_set) — leaving `K` in the frontier with no frontier-confined path. Membership says `reached:true`; the witness BFS dead-ends → self-contradictory evidence (and a latent panic if A3 is written to assume success). The codebase's own `cfg_reachable_including_continuation` hack documents that line-granularity CFG membership has interior-node false-negatives, so this is not theoretical. A single test cannot establish a property the construction does not enforce.
*Resolution:* Replace the two-pass (unconfined-frontier + confined-witness) design with a **single predecessor-tracking forward BFS** that applies the CFG filter *inline during traversal* (a node is reached only *through* a surviving predecessor) — frontier = visited set, `reached` = sink ∈ visited, witness = predecessor walk-back, all invariant by construction. This also collapses the three stitched traversals the spec currently stacks (frontier fan, separate `dfg_witness_path`, separate `apply_cleansers`) into one. Note this is a deliberate *semantic* change (frontier becomes "reachable via an all-CFG-valid path"), so state it. **Alternatively**, explicitly specify the degraded contract (downgrade `reached`, or emit best-effort witness + `Warning`) and forbid A3 from asserting. Settle this §4 decision before planning. *(Soundness headline; Rigor independently flagged witness encoding but missed the existence flaw — complementary, not conflicting.)*

**2. §4/§5/§9 — multi-witness graph encoding is underspecified.**
*Issue:* `max_results` "caps witnesses" without defining witness *identity* (per sink, per source-sink pair, or per path), and the spec never says whether multiple witnesses are unioned into one `GraphPayload`, appended as disconnected paths, deduped/remapped, or linked back to `SinkResult`. Rigor confirms determinism (shortest path, sort order) is now resolved, but multiplicity/composition/linkage remain open.
*Resolution:* Define witness identity, whether witnesses share one graph or are kept separate, dedup/remap rules, and the `SinkResult ↔ graph` linkage.

**3. §6 / M11 — over-cap wire-size policy is still "specify behavior," not a contract.**
*Issue:* The spec punts the actual behavior for frames exceeding the cap. Current `write_message` only serializes/flushes; JSON-RPC protocol errors have no `_meta` path; success shaping is separate.
*Resolution:* Define one exact valid-JSON policy for each of the four frame classes (success, tool `isError`, terminal over-cap, protocol-error), add `anthropic/maxResultSizeChars` to error `_meta`, and anchor the implementation task to the impl bodies `transport.rs:428` / `:462` (the `#[cfg(test)]` impl) — **not** `transport.rs:68`, which is a call site. *(Folds Soundness's citation MINOR; Soundness verified the chokepoint claim itself is sound and already named both impls in prose.)*

**4. §5 vs §9 — MCP registration contract contradicts itself.**
*Issue:* §5 says the tool registers via a separate `reason_v1()` and `nav_v1()` stays frozen at 6 tools; §9 says the registry test changes `6 → 7`. Today `prism-mcp` calls only `ToolRegistry::nav_v1()`, which is the sole constructor.
*Resolution:* Pick one and name the exact constructor `run` uses: keep `nav_v1()==6` plus a combined server registry, **or** make `nav_v1()==7`.

**5. §4 vs §8 — `ReasoningSummary` / wire schema is internally inconsistent.**
*Issue:* §4 says `ReasoningSummary { reached, per_sink, counts }` while §8 spells `source_count, frontier_count`; §4 has `Reason::TaintedBy { cleansed_for }` while §8 includes `source`. Field shapes and serde names diverge.
*Resolution:* Normalize field names/serde names, state whether the summary appears in frontier mode, and whether `TaintedBy` always carries `source`. *(Severity: Soundness verified the additive `Option<ReasoningSummary>` approach is byte-safe and the `TaintedBy`/`WarningKind` routing is sound — so the mechanism is fine; the blocker is purely the self-contradiction.)*

**6. §3/§7 — seed-resolution failure taxonomy is incomplete.**
*Issue:* "Some fail → warnings, all fail → `QueryError`" does not define per-seed outcomes, warning kinds, or aggregation order. Existing warning kinds lack `SymbolNotFound`/`UnsupportedFile`/`LocationOutOfRange`, and the all-sources-fail row omits `LocationOutOfRange` even though it exists in `QueryError`.
*Resolution:* Enumerate per-seed outcomes, the new `WarningKind` variants, and aggregation order. *(Resolves an apparent disagreement: Soundness retracted a compile-time objection here after verifying `enclosing_function` returns `Option` — so the probe **mechanism** is sound; Rigor's surviving concern is the unspecified **taxonomy**, which stands. Not a conflict.)*

**7. §3 — `Symbol` seed → parameter selection is not exact.**
*Issue:* "Function parameters only" is chosen, but the AST-parameter → `VarLocation` mapping is undefined. Current DFG param defs use `function_parameter_names`, skip field-only params via `has_bare_references`, and locate `Def`s at the function start line.
*Resolution:* Specify receiver/destructuring/rest/default handling, access kind, path matching, and the partial-missing-params warning.

**8. §3/§4 — sink granularity is ambiguous.**
*Issue:* `Loc` seeds resolve to *all* `Variable` locations on a line, but `SinkResult` does not say whether mixed resolved variables aggregate by any/all, produce multiple sink results, or become source-sink pairs.
*Resolution:* Define the aggregation rule for multi-variable sink lines.

## MAJOR

**9. §2/§7/§8 — boundary aggregation is ambiguous for multi-source flows.**
*Issue:* For one sink reached intraprocedurally by source A but boundary-only from source B, the spec doesn't say whether `boundary` is also true, whether warnings are per dropped edge/source/sink, or how this affects top-level `reached`.
*Resolution:* Define boundary-flag aggregation and warning granularity for mixed reach.

**10. §4/§8 — cleansing is underspecified and §4 never populates it.**
*Issue:* Two combined gaps. (a) §4's data-flow steps never invoke `apply_cleansers`, yet the frontier arrives with `cleansed_for` empty and §4/§8 intend it populated for Go/Python/JS-TS — a real completeness gap (Soundness). (b) `apply_cleansers` keys on *function-body sanitizer presence* for the single-source fan, so multiple flows from one source-function get identical `cleansed_for`; the spec warns "empty ≠ clean" but never the converse "populated ≠ this-flow-sanitized," and gives no merge rule for a sink reached by multiple sources with different categories (Rigor).
*Resolution:* Add the `apply_cleansers` step to §4; define multi-source merge rules and `CleansedFlow` warning locations; state in §8 that `cleansed_for` is function-body sanitizer *presence* keyed on the source, not per-path evidence.

**11. §5 — `ShapeOptions` / `verbosity` is not a typed contract.**
*Issue:* The shaper seam is said to own witness-vs-frontier shape, edge-label vocabulary, and unreachable-sink behavior, but enum values and field effects are undefined. Current MCP only has `concise|detailed`.
*Resolution:* Either explicitly reuse `concise|detailed` and pin its reasoning-mode effects, or define a new typed shape option.

**12. §4 — `ControlFlow` edge kind is allowed but has no insertion rule.**
*Issue:* The witness relation is DataFlow + same-line assignment propagation, but the edge vocabulary lists `ControlFlow`; current reachability follows no CFG graph edges.
*Resolution:* Remove `ControlFlow` from v1 witness output, or define exactly when it appears.

## MINOR

**13. §2/§11 — hidden coupling on the cross-function bypass quirk (spec is aware; no guard proposed).**
*Issue:* `taint_forward_cfg`'s deliberate cross-`(file,function)` bypass now gets a *second* consumer with the opposite need — `taint_reaches` filters those flows in v1 and re-consumes them in Phase-IP. A future fix making the method properly intraprocedural would silently break the boundary marker and Phase-IP.
*Resolution:* Add a regression test pinning the bypass as a contract and mark the method's doc as load-bearing for two consumers.

**14. Citation anchoring (narrow).**
*Issue:* Spec line numbers are otherwise correct, but `build_result` also exists at `spiral_slice.rs:282`, so the bare name is mildly ambiguous (intended `src/mcp/output.rs:149`). *(The `transport.rs:68`-is-a-call-site point is handled in Blocker 3.)*
*Resolution:* Disambiguate `build_result` by path/line in the plan.

---

**Verdict: not ready to plan — changes required first.** Resolving the Soundness/Rigor verdict split: Soundness is right that the architecture is ~90% sound, but design-soundness ≠ implementation-readiness — the one structural decision (Blocker 1: the M6 witness-existence construction) plus the self-contradictions (Blockers 4, 5) and the underspecified contracts (Blockers 2, 3, 6, 7, 8) all block an independent implementer. Settle the single-BFS-vs-degradation decision and close the eight blockers; the majors/minors can ride the same revision.