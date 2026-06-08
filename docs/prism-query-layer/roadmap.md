# Prism Query/Navigation — Initiative Roadmap

The larger plan behind Plan 1. Extends Prism from a diff-only review slicer into
an LLM/agent codebase-understanding tool, **preserving diff-review throughout**.

- **Spec (Tier 1):** `docs/superpowers/specs/2026-06-07-prism-navigation-layer-design.md`
- **Branch:** `feat/prism-query-layer`
- **Design history:** `docs/prism-query-layer/provenance/`

## Vision (three horizons)

1. **Tier 1 — whole-repo navigation/architecture** (current spec). Reuse the
   existing CPG/call graph; answer `nodes-at`, `callers`/`callees`, `ego-graph`,
   `module-deps`/`repo-map`. Library-first; CLI + MCP adapters. *Additive — zero
   CPG-core logic edits (Option C).*
2. **Tier 2 — seeded reasoning** (next initiative, own spec→plan). The
   differentiated moat: taint-reaches, dataflow-between, impact-of-change,
   what's-missing — driven from a `FocusSet` seed (a diff is one producer). Sits
   on the Tier-1 seams.
3. **Cross-repo / precision** (later). A SCIP/Glean `SymbolResolver` impl behind
   the resolver seam for compiler-accurate defs/refs and cross-repo. Rejected
   (not deferred): vector RAG, whole-repo long-context, CGM-style models.

## Tier 1 plan-of-plans

Each plan ships working, testable software and maps to spec §17's build order.

### Plan 1 — Foundation + `nodes-at`  *(written: `docs/superpowers/plans/2026-06-07-prism-navigation-layer-plan1-foundation.md`)*
- **Scope:** CLI subcommand scaffold (diff-review byte-for-byte compat goldens;
  `subcommand_negates_reqs`), `repo_loader` + `LoadedRepo`, owned
  `NavigationIndex` (`CpgContext::build`, `scope=None`) with nav-local
  `line_range_index` + `name_index`, the `Evidence` serde contract, and
  `nodes-at` runnable as `prism nav nodes-at`.
- **Spec:** §3, §5, §8, §11, §12 · **§17 steps:** 1–3, 5, partial-8.
- **Done:** `prism nav nodes-at` returns JSON `Evidence`; compat goldens green.

### Plan 2 — `callers`/`callees` + `ego-graph`  *(not yet written)*
- **Scope:**
  - `callers`/`callees`: nav-local **qualifier-aware** resolution over
    `CallGraph::CallSite` (`call_site_line`, `qualifier`) using
    `resolve_callees_qualified` — without modifying core `resolve_callers`
    (that's a tracked follow-up). `CalledBy`/`Calls` evidence; `score=1/(1+hop)`.
  - `ego-graph`: bounded BFS over `EgoEdges ⊆ {Call,Return,DataFlow,
    ContainsVariable}` (no `ContainsStatement` — containment is partial),
    `direction ∈ {Out,In,Both}`, cycle-guarded; `{nodes,edges}` output.
- **Spec:** §7, §8 (ego capability) · **§17 steps:** 6–7.
- **Done:** `prism nav callers/callees/ego` return `Evidence`; qualified-import
  and closure/lambda fixtures pass.

### Plan 3 — Cache + `module-deps`/`repo-map` + MCP  *(not yet written)*
- **Scope:**
  - Exact-hit nav cache (separate namespace) with a **`build.rs` grammar-version
    fingerprint** in the key (closes the stale-tree-after-`cargo update` bug).
  - `module-deps`/`repo-map`: call-derived file→file edges (`source:PrismCpg`) +
    labeled `UnresolvedImport` (heuristic; per-language precision tiers — Rust is
    call-derived-only, no import extraction).
  - `prism-mcp` binary (`rmcp`, validated by a spike; stdio JSON-RPC fallback)
    exposing the five queries → `Evidence` JSON. Sequenced last.
- **Spec:** §9, §10, §13 · **§17 steps:** 9–11.
- **Done:** cache hit/miss + grammar-bump invalidation tested; MCP tools return
  `Evidence`; dogfood smoke on this repo.

## Tracked follow-ups (spec §19 — separate slices, own goldens)

Decoupled from Tier 1 so navigation ships without perturbing diff-review:

1. **CPG-core `func_index` re-key** to `(file,name,start_line)` so the *review*
   path is also collision-safe — invasive (cascades cpg/cache/data_flow/
   call_graph/algorithms), needs a reviewed golden re-baseline.
2. **Qualifier-aware core `resolve_callers`** (`call_graph.rs:801` → forward to
   `resolve_callees_qualified`); may shift call-graph-using algorithm output.
3. **`CallStructureExperimental`** DFG-less build profile (needs CPG-core
   function→statement containment + tests).
4. **Class/struct/module CPG nodes** — enables richer `nodes-at`/`ego-graph`
   (v1 returns Function/Variable only).

## Plan 1 review follow-ups (deferred from the holistic code-review)

Hardening/architectural items the final review raised that were intentionally
deferred (the in-scope correctness/contract fixes were applied in-branch):

1. **`NavigationSession`↔`NavigationIndex` binding** — no invariant guarantees
   the index was built from the session's `LoadedRepo`. v1 has a single
   construction path (`run_nav`); add a repo-identity/content-hash check (or a
   checked constructor) when a second construction path or incremental rebuild
   appears.
2. **Shared repo-loading abstraction** — the nav loader is intentionally separate
   from the diff-review pipeline, so the two can disagree on the file universe;
   and nav does **not** load a `TypeDatabase` (fine for Rust dogfood, a gap for
   C/C++ callee resolution). Unify behind a shared snapshot/ignore policy + pass
   the type-db into the nav index when C/C++ precision matters.
3. **`QueryError` plumbing** — `nodes-at` returns `Evidence`+warnings for all
   cases; `LocationOutOfRange`/`UnsupportedFile` are unused. Wire
   `Result<Evidence, QueryError>` when Plan 2 adds symbol/ambiguous-seed queries
   that need to distinguish bad input from valid-empty.
4. **Encapsulate `NavigationIndex`/`NavigationSession` fields** — currently
   public; expose invariant-preserving query methods and privatize once the API
   stabilizes.
5. **Pre-existing: `review`-suite nondeterminism (Taint).** Surfaced by the
   byte-for-byte compat test — `--algorithm taint` (and thus the `review`
   aggregate) is **not** byte-stable (the golden captured in the Linux verify
   container had an empty `Taint` section; the real output includes the slice).
   This is unrelated to the nav layer (the nav changes are additive) but
   contradicts CLAUDE.md's "BTreeMap/BTreeSet everywhere for deterministic
   output" — some collection in Taint's source/sink path is order/seed- or
   environment-sensitive. The compat test now byte-locks only deterministic
   algorithms (leftflow/thin/parentfunction) and smoke-tests `review`. Worth
   root-causing Taint's nondeterminism separately.

## Next initiative — Tier 2 reasoning layer

Its own brainstorm→spec→plan cycle. Reintroduces `FocusSet` (the diff as one
seed among symbol/location/source→sink), migrates the diff-anchored algorithms
(`membrane`/`echo`/…) to consume it, and exposes taint/impact/dataflow as seeded
queries + MCP tools. The original combined vision is preserved at
`provenance/00-claude-original-full-spec.md`. An evaluation harness (A/B vs an
agentic-search baseline) is the gating question for whether the reasoning tools
beat "just let the agent read."
