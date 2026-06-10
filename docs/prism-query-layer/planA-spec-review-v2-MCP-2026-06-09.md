# Merged Spec Review — Plan A: Substrate Hardening (Tier-2)

Both review lenses (Rigor = completeness/ambiguity; Soundness = design/decomposition) completed successfully and both verified their claims against the current tree. The two converge strongly on the same load-bearing slices (A3, A7, A2, A4) and disagree only on severity and remediation approach in a few places, resolved inline below.

---

## BLOCKER

**1. §A3 / Plan B §4.4 — Witness reconstruction is both under-specified *and* semantically forked.**
- *Issue (combined):* Rigor: placement/signature is ambiguous — Plan A says "new method beside `dfg_forward_reachable`," Plan B places `dfg_witness_path` in `taint_reaches.rs`; return type, no-path shape, `WitnessStep` fields, source inclusion, and tie-break are unspecified. Soundness (the sharper, gated-slice finding): the spec's assertion "witness edge relation ≡ frontier edge relation … so a confirmed-reachable sink always reconstructs" silently conflates **two different confinements**. Relation-confinement (DataFlow + same-line assignment, `query.rs:535-562`) satisfies the M6 "always reconstructs" invariant but can present a control-flow-**infeasible** witness; node-set confinement (only CFG-filtered survivors, `cfg_queries.rs:169-187`) makes every step feasible but lets M6 *fail*. The spec treats the two as interchangeable, so the M6 test is underdetermined and an implementer will pick a horn silently.
- *Resolution:* Pick **relation-confinement plus an explicit CFG-feasibility check** — emit a `WarningKind`/shaper annotation when a reconstructed step is not CFG-reachable from the source, rather than presenting it as feasible (the choice that serves the "non-misleading Evidence" goal). Write the M6 invariant *against that choice*. Then fix the location to one doc and specify the full contract: method name, receiver, inputs, return type, no-path value, start-node inclusion, frontier-confinement parameter, `WitnessStep` fields, deterministic tie-break.
- *Why combined as one blocker:* Rigor catches the missing API shape; Soundness catches a correctness fork in the stated invariant. They are the same slice and must be resolved together — the invariant's truth depends on the confinement choice, which depends on the signature.

**2. §A7 — `ReasoningGraphView` is a named-but-undefined load-bearing contract.**
- *Issue:* Plan A says A7 "establishes `ReasoningGraphView`," but Plan B's normative public surface lists `mod.rs`, `seeds.rs`, `taint_reaches.rs`, `shape.rs` and never mentions it (Rigor). An implementer cannot build "the seam" when the two plan docs disagree on what the seam *is*.
- *Resolution (informed by Soundness):* Soundness verified that the design **composes without any new graph engine** — the frontier comes from the production `taint_forward_cfg` and the witness from A3 over the production petgraph; no composite overlay-edge store is needed (Soundness explicitly retracted its own draft blocker on this point). So reconcile the two docs to **one public surface**: either drop `ReasoningGraphView` and make Plan B's `mod/seeds/taint_reaches/shape` surface normative, or define it as a thin view that Plan B actually consumes. This is a documentation/contract reconciliation, not new architecture.

---

## MAJOR

**3. §A4 / Plan B — Cleansing metadata flow underspecified + layering inversion.**
- *Issue:* The raw primitive `taint_forward_cfg` hardcodes `cleansed_for: BTreeSet::new()` (`cfg_queries.rs:189-199`); production enriches it *afterward* via `apply_cleansers` (`taint.rs:~10645/10848`). So Plan B cannot treat the frontier as already-cleansed, and `pub(crate)`-exposing `apply_cleansers`/`function_body_cleansed_for` from `taint.rs` makes the reasoning layer reach *up* into an algorithm module (a real layering smell, since those helpers depend on taint-local `collect_calls`/`call_path_text`/`call_path_matches`/`is_js_ts_language`).
- *Resolution:* State exactly how `apply_cleansers` is invoked on the `FlowPath`s and how categories map into `Reason::TaintedBy` / `CleansedFlow`. Keep A4's **minimal `pub(crate)` for the gate** (smallest production diff → tightest byte-identity proof, `algo_taxonomy_sanitizers*` fixtures unchanged), **document the inversion as temporary**, and pair the eventual relocation into the existing `src/sanitizers/` layer with A2's Production-only extraction (both are "pull shared logic out of `taint.rs`" refactors).
- *Disagreement resolved:* Rigor wanted a new dedicated reasoning-facing API (`cleansed_for_source(...)`); Soundness wanted the minimal `pub(crate)`. **Soundness wins for the gate** — a larger `taint.rs` edit raises the byte-identity proof surface Plan A is explicitly built to minimize. Rigor's dedicated API is the correct end-state, deferred.

**4. §8 vs §4 — Plan A is scoped well beyond its own stated gate.**
- *Issue:* §8 gates Plan B on **A3 + A4 + A7 only**, yet Plan A also carries A2 (`Precision`) and A5 (`?` edge) — the two most production-adjacent, highest-proof-surface slices — neither of which is consumed by intraprocedural v1 (both lenses agree). A6 is output-neutral cleanup.
- *Resolution:* Ship **A3 + A4 + A6 + A7** as Plan A (A6 stays — additive, the petgraph side already collects into a sorted `BTreeSet`, genuinely output-neutral). Spin **A2 and A5 out** as separately-justified work sequenced when their consumers (Phase-IP) land. Explicitly label A2/A5 as non-gating.

**5. §A2 — `Precision::Overlay` is an orphan, and the extraction lacks an implementable contract.**
- *Issue:* Soundness: no v1 slice reads the field-sensitive bindings — seeds resolve to `VarLocation`s, the frontier comes from the production field-*insensitive* Step-5b edges, the witness walks that same petgraph; `Precision::Overlay`'s only real consumer is Phase-IP ("may never be built"). Rigor: even setting that aside, the spec delegates the actual API to the implementer — exact signature, `Binding` fields, ordering key, resolver interface, and output type (`NodeIndex`-free pairs vs CPG-ready node pairs) are undefined, while Step 5b's quirks are real (`build.rs:327-405`; `ast.rs:2734-2750,2925-2969`).
- *Resolution:* Descope A2 to a **`Production`-only extraction** (a clean dedup refactor pinned by the cpg lib tests), and when scheduled, specify the exact Rust contract Rigor lists. Introduce the `Precision` parameter only in the phase that wires its consumer.
- *Disagreement resolved:* Rigor rated this a BLOCKER (unimplementable as written); Soundness rated it MAJOR (non-gating). **Soundness is right that it does not gate Plan B**, so it is not a blocker to the gate — but it must be descoped + fully specified before A2 itself is scheduled. Hence MAJOR.

**6. §A5 — Rust `?` overlay edge is not design-complete.**
- *Issue:* "Synthetic overlay-only edge kind with a return/exit target" omits the edge type, target representation, AST detection rule, direction, relation label, and consuming traversal. Soundness confirms the "nowhere to live" problem the spec gestures at is **specifically A5's edge**, and that intraprocedural `taint_reaches` v1 does not consume it.
- *Resolution:* Mark A5 **non-gating** and defer to Phase-IP (consistent with #4). If kept, fully specify the six attributes above plus which reasoning traversal consumes it.

**7. Proof targets — exact fixtures missing.**
- *Issue:* A2's byte-identity claim and A6's output-neutrality claim lack named characterization/golden fixtures (Rigor).
- *Resolution:* A2 — snapshot the ordered `CpgEdge::DataFlow` pairs before/after extraction, covering first-call-on-line, `.`/`->` base truncation, Use-before-Def fallback, and callee-range lookup. A6 — cycle/diamond tests proving set equality and deterministic parent choice if the witness shares traversal logic.
- *Affirmation (Soundness):* A2's proof correctly targets the **edge set, not Taint output**, and §7 correctly bars claiming byte-identity through the `review` preset (Taint is pre-existing nondeterministic there, `nav_compat_test.rs:17-22`). Keep this — it's a strength; the fixture specifics above are the only gap.

**8. Acceptance bar for intraprocedural-only v1.**
- *Issue:* v1 drops cross-`(file,function)` targets — precisely the substrate's *only* interprocedural edges (Step 5b arg→param) — so "non-misleading Evidence" is validated on a deliberately narrowed slice until Phase-IP (Soundness, reframed from "trivial").
- *Resolution:* Add an explicit acceptance fixture — a multi-step intraprocedural def-use chain with a real sanitizer marker — so the gate is not met by source≈sink-adjacent witnesses alone.

---

## MINOR

**9. §A6 — Method-name and determinism nit.**
- *Issue:* The method is `DataFlowGraph::forward_reachable` (`data_flow.rs:479`), not `reachable_forward` as the spec's finding #5 names it (Rigor; Soundness explicitly took the `DataFlowGraph` half on the spec's word, so Rigor is the authority here).
- *Resolution:* Correct the name, and state "first enqueue wins after neighbors are sorted by `NodeIndex::index()`," so the push-time guard cannot later perturb witness parent selection.

---

**Strengths to preserve:** overlay-first with no `CACHE_VERSION` bump; reusing the production `taint_forward_cfg` for the frontier (cache-respecting); per-slice proof obligations correctly routed around the known Taint nondeterminism; and the spec's own verified self-corrections (sanitizers already wired, A6 receiver naming, Option-C limited to A7). The factual grounding survived both second-pass verifications intact.

---

**Verdict:** Not yet ready for `writing-plans`. Required first: (1) resolve the A3 confinement fork — choose relation-confinement + explicit CFG-feasibility annotation and pin M6 to it, with a complete method contract [BLOCKER]; (2) reconcile A7's `ReasoningGraphView` against Plan B's one public surface [BLOCKER]; (3) cut Plan A to its actual gate (A3+A4+A6+A7), deferring A2's `Precision::Overlay` and A5's `?` edge, and specify A4's cleansing-metadata flow [MAJOR]. The remaining MAJOR/MINOR items are scope-labeling and proof-fixture tightening that can land alongside.