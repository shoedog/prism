Both lenses succeeded (no error markers); the merge below reconciles them. Where they disagreed on severity, the resolution line states which lens prevails and why. Items both lenses agreed were already fixed in revision 2 (the §4/§8 `reached = sink ∈ frontier` "contradiction") are omitted as non-findings.

---

# Merged Spec Review — `taint_reaches` (Plan B), revision 2

## BLOCKERS

**B1 — §3 / §4.6 / §8: cleansing field shape invites a path-level-proof misread (security-critical wire output).**
Issue: Revision 2 warns three times in prose that `cleansed_for` is *function-body sanitizer presence keyed on the source, not per-path proof*, but the emitted data is unchanged — `Reason::TaintedBy { cleansed_for: ["xss"] }` on every tainted item and `WarningKind::CleansedFlow` at the sink. An autonomous agent reads `cleansed_for: ["xss"]` on a taint path as "this path is XSS-safe" — the exact false-negative the prose forbids. Prose lives in the spec; the field lives in the wire output the agent consumes.
Resolution: Encode the weakness in the *shape*, not the prose — e.g. `sanitizers_present_in_source_fn: [...]` plus an explicit `path_proven: false`, so the field cannot be read as path-level. *(Severity resolution: only Soundness caught this, and made it the sole gating condition; because it is security semantics on consumed output, it is elevated to BLOCKER. Rigor was silent here.)*

**B2 — §2 / §3 / §4: API contract split — does `taint_reaches` resolve seeds or receive them resolved?**
Issue: §2's signature `taint_reaches(session, sources: SeedSet, sinks: Option<SeedSet>)` takes the *resolved* `SeedSet` (§3: `SeedSet { seeds: Vec<ResolvedSeed> }`), yet §4 step 1 says it "resolves sources/sinks via `resolve_seed_set`," and the MCP tool receives raw `SeedInput`/`SeedSpec`. The seam between MCP (raw specs) and the library (resolved set) is contradictory.
Resolution: Pick one — `taint_reaches(specs: &[SeedSpec])` that resolves internally, or `taint_reaches_resolved(SeedSet)` plus a thin adapter the MCP layer calls. State which.

**B3 — §3 / §7: `Loc` seed truth table is incomplete and internally inconsistent.**
Issue: §3 says `Loc` with `enclosing_function == None` → `LocationOutOfRange`/`UnsupportedFile`, while §7 says "source resolves but outside any function" → empty `Evidence` + warning. `enclosing_function` returns `None` for *three distinct* cases (missing file, out-of-range line, in-file-but-outside-function), which the spec must disambiguate — these are materially different answers to an agent.
Resolution: Define exact outcomes for each: missing file, skipped/unsupported file, out-of-range line, in-file-outside-function, and line with zero `Variable` nodes. Callers must distinguish the `None` cases rather than collapsing them.

**B4 — §4.2 / §4.6 / §6: `Trace { frontier, parents, boundary }` is underspecified for the outputs it must produce.**
Issue: §6 promises relation-named edges (`"DataFlow"` / `"AssignmentPropagation"`) and source-specific `InterproceduralBoundary` warnings, but `Trace` only holds `frontier`/`parents`/`boundary` (Plan A: `parents = predecessor map`). The spec never says where the edge *relation* comes from (stored in `parents` vs recomputed from the petgraph edge type), nor what fields `boundary` edges carry to name the dropped source.
Resolution: Specify that the relation label is recomputed from the petgraph edge type (or stored), and that each `boundary` edge carries at least source/root, from, and to so the warning can name source B's dropped edge. *(Rigor's gap register correctly softened the original "parents must carry edge relation" claim — the requirement is to define the source of the relation, not to mandate storage.)*

## MAJORS

**M1 — §4.5 vs §8: `SinkResult` → graph-node linkage mechanism is unspecified.**
Issue: §4.5 says each `SinkResult` "carries the sink's `GraphNode` location for linkage," but §8's struct is `{ sink: SymbolRef, reached, boundary }` with no node index.
Resolution: State the linkage method — either add `graph_node: Option<usize>` (index into `GraphPayload.nodes`) or specify match-by-`SymbolRef`→`(file,line,path)` — and define it for unreachable, boundary-only, and clipped sinks. *(Severity resolution: Rigor called this a BLOCKER "contradiction," Soundness a MINOR; Soundness is right it is **derivable** from the `SymbolRef`, so not a contradiction — but Rigor is right the mechanism + edge-case behavior are undefined → MAJOR. Note this shares the `(file,line,path)` key flagged in M2, so fixing M2's key fixes both.)*

**M2 — §4.5: witness dedup key `(file,line,path)` collapses distinct nodes — witness-graph correctness bug.**
Issue: `VarLocation`/`SymbolRef::Variable` carry `function`, `kind`/`access`, and `ordinal`. Deduping nodes by `(file,line,path)` merges a `Def` and a `Use` at the same line (losing the assignment-propagation step that joins them, or creating a self-edge) and can merge same-line symbols across different functions.
Resolution: Use full node identity — `(file, function, line, path, kind/access, ordinal)` — or explicitly state that occurrence-site merging is intended and self-edges are dropped, and how edge labels survive the merge. *(Severity resolution: Rigor BLOCKER vs Soundness MINOR; both flag the same key. It is a real correctness defect in the core witness output (Rigor) but with a mechanical key-change fix (Soundness) → MAJOR.)*

**M3 — §6: wire-size policy names the frame classes but does not define their JSON shapes.**
Issue: §6 now enumerates four frame classes needing a policy (success, tool `isError`, terminal over-cap, protocol-error) and splits `default-run = "prism"` into its own commit — but does not define the truncation-marker JSON, the `_meta` shape, the truncation target, or which seam owns the cap. The seams are confirmed (`transport::write_message`, `error_meta`, `error_response`).
Resolution: Define the exact post-truncation JSON-RPC payload and `_meta` shape per frame class, the truncation target, and cap ownership, as task-0 foundation work. *(Severity resolution: Rigor BLOCKER vs Soundness DROPPED; Soundness is right the four classes are now named and `default-run` is split out, Rigor is right that naming classes ≠ specifying the shapes → unfinished but with identified seams → MAJOR, not a blocker.)*

**M4 — §5 / §7 / §8: reusing nav `Reason`/`WarningKind` enums for reasoning concepts dissolves the nav/reasoning boundary.**
Issue: Each reasoning concept is a flat new variant on a *nav* type (`Reason::TaintedBy`; `WarningKind::{SeedUnresolved, InterproceduralBoundary, CleansedFlow}`; `ReasoningSummary` on `Evidence`). Additivity is byte-safe, but after all four Tier-2 tools land, nothing at compile time prevents a reasoning-only variant from appearing inside a nav-produced `Evidence` (the `other =>` catch-all `{:?}`-dumps it rather than failing to compile).
Resolution: Quarantine the growth behind one nested variant — `Reason::Reasoning(ReasoningReason)` / `WarningKind::Reasoning(..)` — so nav's surface stays closed. Cheap now, expensive after four tools have flat variants; do it before tool #2. *(Unique to Soundness; recommend-not-block.)*

**M5 — §12 / Plan A: dependency-gate mismatch on A6.**
Issue: Plan A declares the implementation gate as A3 + A4 + **A6** + A7; Plan B lists only A3 + A4 + A7.
Resolution: Reconcile the dependency list (add A6 or justify its exclusion) before planning.

**M6 — §3 / §7: Symbol-seed param resolution has undefined edge cases.**
Issue: §3/§7 cover *some* params unresolvable (partial `SkippedPath`), but not: zero-parameter functions, **all** params field-only (all skipped by `has_bare_references` → empty source set), unused params, or all params lacking DFG `Def`s. These sit between "partial skip" and "all sources fail."
Resolution: Add rows for each; for the all-field-only / all-empty case, pick one outcome (consistency suggests empty `Evidence` + warning, matching "source resolves but outside any function"). *(Folds Soundness's all-field-only row into Rigor's broader param edge-case finding.)*

**M7 — §7: all-fail `QueryError` has no deterministic precedence.**
Issue: §7 returns a `QueryError` when all sources (or all sinks) fail, but gives no precedence when failures differ (`AmbiguousSymbol`/`SymbolNotFound`/`UnsupportedFile`/`LocationOutOfRange`), and forbids new aggregate variants.
Resolution: Define a fixed precedence or use seed-input order to select the reported error.

**M8 — §3 / §4.3: cleansing-helper contract is unspecified.**
Issue: The `pub(crate)` `apply_cleansers`/`function_body_cleansed_for` seam returns early on empty paths, and the `cleansed_for` category string vocabulary/casing is not pinned. (Distinct from B1, which is about the field *shape*; this is the helper internals + category vocabulary.)
Resolution: Define a source-location-keyed helper variant and the exact category strings (names + casing) that may appear in `cleansed_for`.

**M9 — §4.4 / §8: output determinism is under-defined.**
Issue: Only "sources at score `1.0`" is pinned, yet `EvidenceItem.score: f32` is required for *every* frontier item — the downstream/non-source score is undefined (uniform `1.0`? distance-decayed like `gradient_slice`?). Tie-break tuple, and whether `source_count`/`frontier_count` are reported before or after caps, are also unspecified.
Resolution: Define the non-source score rule, the full deterministic tie-break tuple, and the pre/post-cap semantics of the summary counts. *(Merges Rigor's determinism finding with Soundness's frontier-score finding — same gap.)*

## MINORS

**m1 — §4.5 / §8: `max_results` semantics conflict with "one shortest witness per reached sink."**
Issue: If exactly one shortest witness is kept per reached sink, it is unclear what `max_results` caps (frontier items, sinks, paths, or graph nodes).
Resolution: State the cap's unit, and require `SinkResult`/graph linkage to stay complete after clipping.

**m2 — §3 / §9: params-only reach scope is not disclosed in the MCP tool description.**
Issue: "Taint from function F" = F's parameters only; taint entering via a local/global/env read inside F is unrepresentable and yields `reached: false`. §9's tool surface doesn't say so, and §8's top-level `reached` is `Some(any sink reached)` — an agent reading only the top bool over-trusts both the negative (scope artifact) and the positive (one of N sinks).
Resolution: State the params-only scope in the `reason_taint_reaches` description and steer agents to `per_sink`, not the top-level `reached`.

**m3 — §4.6 / §8: multi-source `cleansed_for` picks the shortest witness's source — path length, not sanitization.**
Issue: A longer-but-sanitized path loses to a shorter-but-unsanitized one; the agent sees an arbitrary (if deterministic) security signal. Compounds B1.
Resolution: Union `cleansed_for` across witnessing sources, or drop per-sink cleansing in witness mode and surface it per-source only.

**m4 — §7 / §8: `WarningKind::SeedUnresolved { spec, reason }` JSON shape is unpinned.**
Issue: A struct variant serializes differently from the existing unit `WarningKind` variants, and `Warning` already carries `message`/`location`.
Resolution: Pin the exact JSON shape (and reconcile with the existing `Warning` fields).

**m5 — §9: MCP schema omits defaults and array constraints.**
Issue: The hand-authored schema lacks `minItems`, default `max_results`/`verbosity`, and the meaning of `sinks: []`.
Resolution: Add `sources.minItems = 1`, decide whether empty `sinks` means invalid or frontier mode, and state the defaults.

**m6 — §10 / Plan A §7: S→I→K invariant is an execution checkpoint, not a spec change.**
Issue: The invariants ("reached ≡ sink ∈ frontier"; every frontier member has a dead-end-free witness) depend on Plan A's A3 single inline-CFG-filtered BFS, which is not yet on `main` (today's `taint_forward_cfg` is the node-wise over-approximation that self-contradicts on S→I→K). The spec already gates on A3 and lists the S→I→K test in both Plan A §7 and Plan B §10.
Resolution: No spec change — confirm during execution that the S→I→K test is written test-first and is a must-pass-before-A3-is-done blocking contract, not merely a listed target. *(Both lenses agree this is a dependency/execution gate, not a Plan B defect; Soundness's earlier blocker is correctly downgraded.)*

---

**Verdict: Not ready to plan as-is.** The architecture is sound and unusually well-grounded (nearly every file:line reference survives verification, and revision 2 already absorbed much of a first pass), but four blockers must be resolved first — lead with the cleansing-field reshape (B1, security-critical), then the API contract split (B2), the `Loc` truth table (B3), and the `Trace`/boundary shape (B4) — plus align the A6 dependency (M5) and, before tool #2, the nested-variant enum quarantine (M4). Sweep these and the MINOR completeness items in one spec-editing pass, then proceed to `writing-plans`.