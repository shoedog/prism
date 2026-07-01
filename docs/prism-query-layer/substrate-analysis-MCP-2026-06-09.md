> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

I'll merge these two designs. Both architects converge strongly, and most of their "disagreements" are actually one giving a sharper version of the other's claim. Before writing the merged design, let me adjudicate the three load-bearing factual claims the merge pivots on — these determine the central architectural recommendation.All three load-bearing facts are adjudicated. Results:

- **`CpgEdge::Return` exists** (types.rs:90) — claude's G2 is correct. *Correction to claude's specifics:* the predicates are named `is_interprocedural()` (matches `Call | Return`, types.rs:168) and `is_data_flow()` (matches `DataFlow` only, types.rs:163), **not** `is_call_like`/`is_data_like`. Substance holds, names drifted.
- **Step 5b field-insensitivity confirmed** (build.rs:367-368): `split('.').next()` → `split("->").next()` → `AccessPath::simple(arg_base)`, emitting only `CpgEdge::DataFlow` (build.rs:400), arg→param only, no return. Both architects correct; this is the real central soundness defect.
- **`resolve_dispatch` is fully dead** — zero call sites in `src/`, only the 5 provider impls + trait + 2 doc comments. Claude's G3 (sharper than codex's "unused for edges") is correct.
- **Six-tool test + single `nav_v1()` factory confirmed** (registry.rs:55, :79) — claude's G4 mechanism is correct.

Here is the merged design.

---

# Merged Design — Deepening / widening / hardening the prism reasoning substrate (DFG · CFG · CPG)

## Approach (the thesis both architects reached independently)

The one block where data flow crosses a function boundary — **Step 5b in `src/cpg/build.rs:327-405`** — is simultaneously the *only* interprocedural flow, *return-blind*, *field-insensitive* (collapses every argument to its base identifier at build.rs:367-368), *name-resolved*, and *untestable in isolation*. Everything Tier-2 needs (`taint_reaches`, `dataflow_between`, `impact_of_change`, `what_missing`) is gated on fixing that block.

The synthesis: **make Tier-2 an additive reasoning *overlay* over the existing CPG, not a mutation of the persisted CPG/DFG/CFG/nav output.** Extract the inline binding logic into a pure, separately-tested function; let the persisted CPG keep emitting byte-identical edges; let a new `src/reasoning/` layer compute the *richer* flows (return→caller, field-sensitive args, typed dispatch, classified sanitizers) into an ephemeral graph view that only Tier-2 tools see. This satisfies Option-C additivity *by construction* and defers every golden/cache re-baseline to a deliberate, measured, opt-in step.

## Convergent spine (both chose independently → high confidence)

1. **Tier-2 is additive over the existing CPG.** No changes to DFG/CFG/CPG output or the six nav tools. Option C preserved.
2. **New `src/reasoning/` module** is the consumer layer.
3. **Return→caller flow is the headline missing seam.** AST return extraction already exists (codex: ast.rs:1943); Step 5b never consumes it.
4. **Interprocedural binding gets extracted out of the graph-assembler** into something testable in isolation.
5. **Structured AST helpers reusing `AccessPath`** for argument/return path extraction (codex: `call_argument_paths_at` keyed on the call node; claude: `AccessPath::from_expr`).
6. **Typed call resolution rides the existing-but-unused dispatch providers**, metadata-first / trait-based; keep name-based resolution as default; do not delete name edges until measured.
7. **CFG gains typed edge metadata as a *parallel* API**; existing line-pair output and generic `ControlFlow` emission unchanged.
8. **`cleansed_for` is always empty today** (data_flow.rs:610) and must be populated for sound taint — via a *new* classified taint path, leaving `taint_forward` unchanged.
9. **Reasoning MCP tools register on a separate surface** — the six nav tools and their pinned test stay frozen.
10. **`what_missing` is the weakest tool — sequenced last.**
11. **Characterization/construction tests precede any behavior change**; cache needs a version bump / capability fingerprint *if* new edges are ever persisted.
12. **The biggest soundness risk is interprocedural precision** — codex frames it as call-resolution false paths, claude as field-insensitive binding. These are two faces of the same gap.

## Architecture — components & file boundaries

```
src/data_flow/
  interproc.rs   (NEW)  compute_bindings(cg, files, &dyn CallResolver) -> Vec<Binding>   [pure, no petgraph]
  classify.rs    (NEW)  trait FlowClassifier; taint_forward_classified(...) populates cleansed_for
  (data_flow.rs)        + same-line index INSIDE DataFlowGraph (S0 perf fix)
src/call_graph/
  resolve.rs     (NEW)  trait CallResolver; NameResolver (default), TypedResolver (gated)
  (call_graph.rs)       + CallSite.receiver: Option<String>  (additive field)
src/ast.rs              + call_argument_paths_at / call_result_lvalue_paths_at / return_value_paths_in_function
src/cfg.rs              + build_typed_cfg_edges() -> Vec<TypedCfgEdge> (parallel API; old fn projects from it)
src/cpg/build.rs        Step 5b DELEGATES to compute_bindings (behavior-preserving) — emits identical DataFlow
src/reasoning/   (NEW)
  graph.rs              ReasoningGraphView: overlay of ReasoningEdge over CPG node indices + evidence
  path.rs               bounded deterministic BFS -> FlowEvidence { steps, warnings, truncated }
  context.rs            ReasoningContext / ReasoningIndex (wraps NavigationIndex)
src/mcp/
  registry.rs           + reason_v1() / register_reasoning(...)  (nav_v1() and its 6-tool test FROZEN)
  reason_tools.rs (NEW) reason_taint_reaches, reason_dataflow_between, reason_impact_of_change, reason_what_missing
```

### Key types

```rust
// src/data_flow/interproc.rs — pure, deterministic (cg.calls is BTreeMap, sites are BTreeSet)
pub enum BindingKind { ParamIn, ReturnOut }     // ReturnOut = the new capability
pub struct Binding {
    pub kind: BindingKind,
    pub caller: FunctionId, pub callee: FunctionId,
    pub call_site: CallSiteKey,                  // keyed on the call NODE, not (line,name) — fixes G6
    pub arg_index: usize,                        // POST-receiver-offset — fixes G5
    pub from: AccessPath, pub to: AccessPath,    // field-sensitive via AccessPath (deferred flip, see S6)
}
pub fn compute_bindings(cg: &CallGraph, files: &BTreeMap<String,ParsedFile>,
                        resolve: &dyn CallResolver) -> Vec<Binding>;

// src/reasoning/graph.rs — ephemeral overlay; NOT persisted, NOT in nav output
pub enum ReasoningEdgeKind {
    IntraDataFlow, ArgToParam, ReturnToCaller, ParamFieldProjection,
    TypedCall { confidence: CallResolutionConfidence },
    Control { kind: CfgEdgeKind }, Sanitizer { category: SanitizerCategory },
}
pub struct ReasoningContext<'a> {
    pub cpg: &'a CodePropertyGraph,
    pub files: &'a BTreeMap<String, ParsedFile>,
    pub types: &'a TypeRegistry,          // codex: surfaces the registry CPG construction never wires in
    pub live_types: &'a BTreeSet<String>, // for TypedResolver dispatch pruning
    pub skipped: &'a [SkippedFile],       // codex: surface loader quality signals as warnings
}
```

### The flow

`compute_bindings` is the single source of truth for interprocedural edges. `build.rs` calls it to reproduce **today's** arg→param `DataFlow` edges (proven byte-identical by a characterization test). The `ReasoningGraphView` *also* calls it — plus the new AST return/field helpers — to materialize `ReturnToCaller` and field-sensitive `ArgToParam` as **overlay edges that never touch the persisted CPG**. `reason_*` tools run `path.rs` BFS over the overlay and emit `FlowEvidence`.

## Key decisions (where the two designs diverged → I picked / integrated)

| # | Divergence | Decision | Why |
|---|---|---|---|
| **D1** | New `CpgEdge` variants in the CPG (claude) vs separate `ReasoningGraphView` overlay (codex) | **Overlay-first.** New flows live in `src/reasoning/`, not the persisted CPG. New `CpgEdge::ReturnFlow`/`Throws` become a *deferred, measured promotion* — see Owner Decision #1. | Codex's overlay satisfies Option-C *by construction*: no cache serialization change, no nav-ego default drift, nothing to re-baseline. It directly neutralizes claude's own G8 ("golden impact unmeasured"). Claude's predicate-hygiene insight (G2) is retained as the guard rail *for the promotion path*. |
| **D2** | Extract Step 5b verbatim + characterization test (claude S1a) vs "do it in the reasoning view first" (codex) | **Both, unified:** extract into pure `compute_bindings`; `build.rs` delegates to reproduce identical edges (characterization test); overlay reuses the same function for richer flows. | Avoids duplicating arg→param logic in two places, gives claude's byte-identity proof, and gives codex's reusable helper. One function, two callers. |
| **D3** | Typed dispatch is "free precision" (codex) vs "dead code, gate it" (claude G3) | **Gate it.** `TypedResolver` ships *with* per-provider `resolve_dispatch` characterization tests as a hard precondition; `NameResolver` stays default. | **Verified:** `resolve_dispatch` has zero call sites in `src/`. The path has literally never executed — correctness is unproven, not free. |
| **D4** | "Register tools separately" (codex) vs concrete `register_reasoning`/`reason_v1()` keeping the 6-tool test frozen (claude G4) | **Claude's mechanism + codex's `reason_` naming.** | **Verified:** there is a single `nav_v1()` factory (registry.rs:55) and a test pinning exactly six tools (registry.rs:79). A parallel `reason_v1()` surface leaves both untouched. |
| **D5** | `compute_bindings` arg alignment unaddressed (codex) vs receiver-offset + call-node keying (claude G5/G6) | **Adopt claude's correctness boundaries**, implemented via **codex's call-node-keyed AST helper**. | Codex's `call_argument_paths_at(call_key)` *is* the fix for claude's G6 (today's `call_argument_texts` keys on `(line, callee_name)`, build.rs:340). Receiver offset (G5) is added on top. Complementary, not competing. |
| **D6** | Sanitizer/`cleansed_for` (both) | New `taint_forward_classified` + `FlowClassifier` trait. | Both agree `cleansed_for` empty (data_flow.rs:610) makes taint *reachable but not sound*; a classifier is the only thing that fills it. |
| **D7** | S0 quadratic-scan perf fix (claude-only, G1) | **Include, in the corrected layer.** | Codex didn't surface it; claude self-corrected the layer — the same-line index must live **inside `DataFlowGraph`** (forward_reachable iterates `self.defs`, data_flow.rs:498), not reuse the CPG's `location_index`. |
| **D8** | Parameter field projection (codex-only) | **Include in interproc ordering.** | Codex-unique: the DFG intentionally skips base-parameter defs when the callee only uses `param.field`, so binding must project. A real precision step claude missed. |

**Predicate-name correction (carry into any promotion work):** if `ReturnFlow` is ever added, the predicates to update are `is_interprocedural()` (types.rs:168) and `is_data_flow()` (types.rs:163) — **not** the `is_call_like`/`is_data_like` names claude cited (those don't exist). `ReturnFlow` must classify as data, and a test should assert `ReturnFlow.is_interprocedural() == false`.

## Risks

1. **Field-sensitive binding re-baselines diff-review (highest likelihood, if/when promoted).** Trigger: any diff with `f(cfg.name)`. *Mitigation:* the overlay-first design keeps this out of the persisted CPG entirely until S6; the S1a characterization test converts G8 from "reasoned" to "measured" before any flip.
2. **Interprocedural call-resolution false paths (the shared #1 soundness risk).** Name/qualifier fallback (`resolve_callees_qualified`, build.rs:330) over-approximates; field-insensitivity (build.rs:367-368) manufactures false flows (`cfg.debug` taint arrives as taint on all of `cfg`). *Mitigation:* keep `NameResolver` default, label every candidate's confidence, emit ambiguity warnings, measure before relying.
3. **`TypedResolver` rides never-executed code (verified).** *Mitigation:* per-provider characterization tests gate S2; opt-in for reasoning only.
4. **Receiver-offset / same-line duplicate-call mis-binding (G5/G6).** *Mitigation:* `arg_index` is post-receiver-offset; bind off the call node, warn on `(line,name)` ambiguity.
5. **Interprocedural/recursion blowup.** *Mitigation:* bounded depth in `path.rs`, reuse SCC machinery, emit a truncation warning.
6. **`cfg.rs` `?`/early-return is a genuine hole** (the dispatch comment over-claims it's handled). Tracked under S6/S3.

## Smallest shippable slices + build order

1. **S1a — Extract Step 5b into pure `compute_bindings`; `build.rs` delegates.** Ships *with* the characterization test asserting byte-identical `CpgEdge::DataFlow` output. Pure refactor, golden-neutral, unblocks everything. *(Resolves G8 by measurement.)*
2. **S0 — Parallel hardening:** relocate the same-line index *inside `DataFlowGraph`* (G1); emit a `ParseQuality` warning where CFG edges silently drop and where binding falls back to `(line,name)`. Additive.
3. **S-ast — Structured AST helpers** (`call_argument_paths_at` keyed on call node, `return_value_paths_in_function`, `call_result_lvalue_paths_at`).
4. **S-overlay — `src/reasoning/graph.rs` + `path.rs`:** `ReasoningGraphView` consuming `compute_bindings`; stepwise intraprocedural `dataflow_between`. First reasoning capability, zero CPG/cache/nav change.
5. **S1b — Return→caller + field-sensitive args as *overlay* edges** (`ReturnToCaller`, field-sensitive `ArgToParam`). Still overlay-only.
6. **S2 — `CallResolver` trait + `NameResolver` default;** add `CallSite.receiver`. Then `TypedResolver` *gated on per-provider `resolve_dispatch` characterization tests*, opt-in.
7. **S4 — `FlowClassifier` + `taint_forward_classified`** populating `cleansed_for`.
8. **S-tools — Register `reason_*` via separate `reason_v1()`;** ship `reason_taint_reaches` + `reason_dataflow_between` first (exercise S1b+S2+S4), then `reason_impact_of_change` (S2 reverse). `nav_v1()` + 6-tool test untouched.
9. **S5 — `reason_what_missing`,** spec'd separately (splits into echo-style *missing-error-handling* needing return-flow+CFG, vs absence-style *missing-counterpart* needing neither). Last.
10. **S6 (deferred, deliberate, isolated) — Promotion + CFG fidelity:** *if* measurement justifies it, promote overlay edges into the persisted CPG (`CpgEdge::ReturnFlow`/`Throws` + predicate hygiene + `CACHE_VERSION` bump + reasoning capability fingerprint), and land CFG exception/`?` edges. The two announced golden re-baselines, alone.

---

## DECISIONS FOR THE OWNER

1. **Overlay-only forever, or eventual promotion of reasoning edges into the persisted CPG?**
   - *Overlay-only (recommended start):* zero Option-C risk, no cache/golden churn, recomputed per query. Codex's position.
   - *Promote later:* persisted + traversable with existing petgraph machinery + warm-cache reuse, but requires `CACHE_VERSION` bump, a reasoning capability fingerprint, predicate hygiene (G2), and a measured golden re-baseline. Claude's position.
   - **Recommendation:** start overlay-only; promote *only if* reasoning-query latency on warm caches proves it necessary, and only behind the guards above. Sequence as S6.

2. **Build `TypedResolver` in the near term, or defer until name-based precision is measured insufficient?**
   - Both architects want typed dispatch, but it rides verified-dead `resolve_dispatch`. Building it now means writing 5 provider characterization-test suites up front.
   - **Recommendation:** land S2's `CallResolver` trait + `NameResolver` now (cheap, unlocks the seam); build `TypedResolver` only after S-tools exists to *measure* name-based false-path rate — then you know whether the test-gate cost is justified.

3. **Reasoning tool naming: `reason_*` prefix (codex) or bare `taint_reaches`/`dataflow_between` (claude, matches the brief)?**
   - **Recommendation:** `reason_*` prefix, for namespace symmetry with `nav_*` on a separate registry surface. Minor; owner's call.

4. **(Minor) `CpgEdge::FieldOf` is declared and exposed to nav/MCP but has no construction site** (codex). Decide: populate it from `AccessPath` field structure as part of reasoning, leave it as documented-latent, or remove it. Not blocking.

**Readiness verdict:** Ready to plan after deciding the two substantive open questions (overlay-only vs promotion; TypedResolver now vs measure-first) — the naming and `FieldOf` calls can be made in-flight. S1a is unambiguous and can start immediately.
