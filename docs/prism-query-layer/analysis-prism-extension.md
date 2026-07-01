# LLM Codebase Navigation: Prism Extension Analysis

> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

**Status:** Analysis  
**Date:** 2026-06-06  
**Scope:** Evaluate extending Prism to support LLM-oriented codebase navigation and understanding techniques while preserving the existing diff/code-review use case.

## Summary

Prism is already a strong base for graph-assisted LLM code understanding because it has a polyglot tree-sitter parser, a unified Code Property Graph (CPG), data-flow, call graph, control-flow, type providers, scoped construction, caching, and diff-aware slicing algorithms.

The highest-confidence extension is to add an opt-in repo-wide navigation and localization layer on top of the current CPG infrastructure. This should not replace the current diff-review pipeline. The existing `--repo` + `--diff` behavior should remain the default contract.

The recommended direction is:

1. Add repo-wide ingestion/indexing as a separate mode.
2. Add LLM-facing graph navigation commands or JSON APIs.
3. Add hierarchical localization over graph and lexical evidence.
4. Optionally integrate SCIP/Glean/Sourcegraph-style indexes as precision backends for definitions/references, especially cross-repo.

Do not prioritize pure vector RAG, whole-repo long-context prompting, or model-training-heavy CGM-style work as the first step.

## Source Context

This analysis is based on:

- The local research note: `LLM Codebase Navigation & Understanding.md`
- Prism's current implementation, especially:
  - `src/cpg.rs`
  - `src/ast.rs`
  - `src/call_graph.rs`
  - `src/data_flow.rs`
  - `src/cpg_cache.rs`
  - `src/main.rs`
  - `src/algorithms/mod.rs`
  - `docs/features/cpg/architecture.md`
  - `PLAN.md`
  - `LLM.md`

The research note's central finding is that effective LLM codebase understanding favors a layered stack: precise symbol/graph navigation plus lexical search as the backbone, hierarchical localization on top, and semantic embeddings only as a supplemental fallback.

## Current Prism Fit

Prism already has many of the components that RepoGraph- and LocAgent-style systems need:

- Polyglot parsing via tree-sitter.
- A unified CPG with function, statement, and variable nodes.
- Typed graph edges for data flow, control flow, calls, returns, containment, and field relationships.
- Call graph and data-flow graph retained inside the CPG for compatibility and specific query needs.
- Type provider infrastructure across several languages, with C/C++ enrichment through `compile_commands.json`.
- Diff-aware slicing algorithms for code review and security review.
- Scoped CPG construction for changed files plus direct caller/callee neighborhoods.
- CPG caching and incremental rebuild support.
- Structured JSON/review output usable by LLM reviewers.

This means Prism should not start from a generic repo map. Its differentiator is that it can expose review-oriented graph context, taint paths, impact paths, and structural findings rather than only symbol neighborhoods.

## Main Gap

Prism is currently optimized for this workflow:

```text
diff -> parse referenced files -> build CPG/context -> run slicing algorithm -> emit review context
```

It is not yet a general LLM navigation backend. The main gaps are:

- The CLI parses only files referenced by the diff.
- CPG cache semantics are per current source file set, not a reusable whole-repo index.
- Navigation output is narrow. `--format callers` exists, but there are no first-class commands for symbol lookup, definitions, references, ego graphs, nodes at a location, or graph neighborhoods.
- There is no hierarchical localization pipeline that ranks likely files/functions/lines for an arbitrary natural-language or issue-style query.
- Definition/reference precision is still mostly tree-sitter and heuristic import/type resolution, not compiler-accurate SCIP/Glean-level indexing.

## Preserve Diff Review

Items 1-3 can be added without losing the current diff/code-review use case if they are implemented as parallel opt-in capabilities.

Keep the current behavior:

```bash
prism --repo . --diff changes.patch -a review
```

as the stable review path.

The new capabilities should use separate modes:

```bash
prism nav --repo . ...
prism localize --repo . ...
```

or equivalent flags that do not change the existing slicing contract.

Recommended guardrails:

- Do not make repo-wide indexing required for diff review.
- Keep diff-scoped parsing and `--scoped-cpg` intact.
- Use a separate whole-repo cache namespace.
- Keep navigation output advisory until it is benchmarked.
- Do not automatically filter review slices through localization unless the user opts in.
- Add tests that prove legacy diff review output still works when navigation features are present.

## Proposed Architecture

### 1. Repo-Wide Ingestion And Indexing

Add a shared repository loader module, for example `src/repo.rs`, with explicit load modes:

```rust
enum RepoLoadMode {
    DiffFiles,
    ScopedDiffNeighborhood,
    WholeRepo,
}
```

Existing review CLI uses `DiffFiles` by default and `ScopedDiffNeighborhood` when `--scoped-cpg` is enabled. Navigation and localization use `WholeRepo`.

The whole-repo loader should:

- Walk the repository.
- Include supported language extensions only.
- Exclude obvious generated/vendor/build paths.
- Parse files into `ParsedFile`.
- Build a `CpgContext`.
- Cache by repo path, Prism version, file set, file hashes, type enrichment state, and relevant configuration.

This should be separate from the existing per-diff cache so code review remains lightweight.

### 2. Navigation Commands Or APIs

Add a `src/navigation.rs` module that exposes stable query operations over `CpgContext`.

Initial operations:

- `symbols`: list or search known functions/classes/structural units.
- `definition`: return a symbol definition location.
- `references`: return call sites and variable/reference locations where available.
- `callers`: return reverse call graph traversal.
- `callees`: return forward call graph traversal.
- `nodes-at`: return CPG nodes at `file:line`.
- `ego-graph`: return a bounded graph neighborhood around a symbol or location.
- `dataflow`: return forward/backward data-flow neighborhood.
- `chop`: return paths/intersection between source and sink.

Example CLI shapes:

```bash
prism nav --repo . symbol process_request --format json
prism nav --repo . callers src/api.py:42 --depth 2 --format json
prism nav --repo . ego src/api.py:42 --hops 2 --edges call,dataflow,control --format json
prism nav --repo . dataflow src/api.py:42 --direction forward --format json
```

Outputs should be compact JSON with source locations, snippets only when requested, and evidence fields that explain why each location was returned.

### 3. Hierarchical Localization

Add a localization pipeline, for example `src/localize.rs`, with a coarse-to-fine structure:

```text
query -> candidate files -> candidate functions/classes -> candidate lines -> evidence package
```

Start deterministic and simple:

- Lexical/BM25-like matching over file paths, function names, identifiers, call names, and nearby code text.
- CPG expansion from lexical anchors.
- Call/data/control-flow neighborhood scoring.
- Diff-compatible output that can later feed `--files` or review algorithms.

Example:

```bash
prism localize --repo . --query "request body reaches SQL query without parameters" --format json
```

Return ranked candidates:

```json
{
  "query": "...",
  "candidates": [
    {
      "file": "src/api.py",
      "function": "handle_request",
      "line_range": [20, 45],
      "score": 0.82,
      "evidence": [
        {"kind": "lexical", "text": "request"},
        {"kind": "dataflow", "from": "request.body", "to": "execute"},
        {"kind": "call", "callee": "execute"}
      ]
    }
  ]
}
```

The first version should be advisory only. It should not automatically alter review slices until it has an internal evaluation.

## Comparison To Existing Systems

### RepoGraph

RepoGraph is a line-level repository graph used as plug-in context for LLM agents and repair frameworks.

Prism would be similar in using graph-retrieved context, but different in graph shape:

- RepoGraph emphasizes line-level symbol definition/reference dependencies.
- Prism has richer review semantics: functions, statements, variables, data flow, control flow, call/return, field access, taint and review algorithms.

Prism should borrow RepoGraph's idea of graph neighborhoods as prompt context, but does not need to replace its CPG with a line-only graph. A line-level export view may be useful for LLM prompt packaging.

### LocAgent

LocAgent is the closest conceptual match for `prism localize`: graph-guided localization over files/classes/functions with an agent traversing the graph.

Prism can implement the deterministic graph backend and ranking pipeline first. An LLM agent can later call these tools, but Prism should not initially depend on agentic control flow to produce useful localization.

LocAgent is a strong design reference, not a drop-in replacement for Prism's review/security slicing.

### CGM

CGM is a graph-integrated LLM/model-training approach. It uses learned graph representations, graph-aware attention, fine-tuning, retrieval, reranking, and model serving.

This is not a near-term fit for Prism if the goal is to enhance navigation while preserving maintainable static-analysis tooling.

CGM's upside is learned graph reasoning and strong benchmark results. Its downside is a much heavier engineering surface:

- model training/fine-tuning,
- checkpoint management,
- serving costs,
- explainability challenges,
- harder integration with deterministic review findings.

Prism should not try to become CGM unless there is a separate model-training objective.

### SCIP / Sourcegraph / Glean

SCIP, Sourcegraph, and Glean are better viewed as complements than replacements.

Advantages:

- More precise definitions/references.
- Better cross-repo and package-version-aware symbol resolution.
- Mature indexing and incrementality stories.
- Stronger fit for compiler/build-coupled languages.

Limitations relative to Prism:

- They do not provide Prism's review slices.
- They do not directly provide taint/security findings, absence/symmetry/contract checks, or CFG-aware review context.
- They introduce infrastructure, permissions, and indexing complexity.

Best use: add an optional precision backend so Prism navigation can ask an external index for exact definitions/references when available, while falling back to Prism's local CPG.

## Build On Prism Vs Adopt Existing Tools

### Build Onto Prism

Pros:

- Preserves current diff-review workflow.
- Reuses existing CPG, CFG, DFG, call graph, type providers, cache, and algorithms.
- Produces deterministic, explainable evidence.
- Strongest fit for security and change-impact review.
- Can be adopted incrementally.

Cons:

- Requires repo-wide loader and cache work.
- Requires designing stable navigation/localization output contracts.
- Initial symbol precision will remain weaker than SCIP/Glean.
- Cross-repo support remains a separate problem.

Verdict: best default path.

### Adopt RepoGraph

Pros:

- Validated graph-context direction.
- Line-level graph context maps naturally to LLM prompts.
- Good design source for graph neighborhood retrieval.

Cons:

- Less aligned with Prism's current CPG and review algorithms.
- Does not preserve Prism's slicing/security semantics by itself.
- Integration may duplicate graph-building effort.

Verdict: use as design inspiration, not replacement.

### Adopt LocAgent

Pros:

- Closest to hierarchical localization goals.
- Strong conceptual match for file/function localization.
- Useful reference for graph traversal and multi-hop localization.

Cons:

- Focuses on localization, not deterministic review slicing.
- Agent/model workflow may be heavier than needed.
- Would still need integration with Prism's review outputs.

Verdict: use as a blueprint for `prism localize`; do not replace Prism.

### Adopt CGM

Pros:

- Strong graph-integrated model direction.
- Learned reranking and graph reasoning could outperform deterministic heuristics on some benchmarks.

Cons:

- Heavy model-training and serving project.
- Harder to explain and debug than static graph evidence.
- Overkill for navigation API and review context extraction.
- Highest maintenance cost.

Verdict: not a near-term fit.

### Adopt SCIP/Glean/Sourcegraph As Backbone

Pros:

- Best definition/reference precision.
- Better cross-repo story.
- Mature org-scale indexing approaches.

Cons:

- Does not replace Prism's code-review/security analysis.
- Requires external infrastructure or index generation.
- May be unavailable in local/offline workflows.

Verdict: best optional precision backend.

## Recommended Roadmap

### Phase 1: Keep Review Stable, Add Repo Loader

- Add `src/repo.rs` with explicit load modes.
- Preserve existing `main.rs` review behavior.
- Add whole-repo file discovery with ignore rules.
- Add a separate whole-repo cache namespace.
- Add tests for loader behavior and review compatibility.

### Phase 2: Add Navigation API

- Add `src/navigation.rs`.
- Implement `symbols`, `nodes-at`, `callers`, `callees`, and `ego-graph`.
- Expose as CLI JSON.
- Add focused integration tests over small multi-file fixtures.

### Phase 3: Add Hierarchical Localization

- Add `src/localize.rs`.
- Start with lexical + graph scoring.
- Return ranked file/function/line evidence.
- Keep output advisory.
- Add benchmarks against known fixtures and existing diffs.

### Phase 4: Optional Precision Backends

- Add adapter interfaces for external symbol indexes.
- Support SCIP/Sourcegraph/Glean-style definition/reference lookups where configured.
- Fall back to Prism local CPG when external indexes are absent.

### Phase 5: Evaluation

- Compare against the current agentic-search baseline.
- Measure localization precision/recall, token savings, wall time, and review-finding quality.
- Track results separately for Python, JS/TS, Go, Java, Rust, C/C++, Terraform, and Bash.

## Open Questions

- What repo-size threshold should trigger whole-repo indexing warnings or require explicit confirmation?
- What ignore rules should be default: `.gitignore`, hard-coded vendor/build directories, or both?
- Should navigation be a subcommand (`prism nav`) or a format/mode on the existing CLI?
- Should `localize` accept natural language only, or also structured hints like `--symbol`, `--file`, and `--sink`?
- How much source text should navigation output include by default?
- What internal benchmark should define success before localization can affect review slicing?
- Which external precision backend, if any, should be targeted first: SCIP files, Sourcegraph MCP/API, or Glean?

## Bottom Line

Build the LLM navigation layer onto Prism. Prism's existing CPG and review algorithms make it a better base for code-review-oriented navigation than adopting a generic repo graph or localization agent wholesale.

The most practical architecture is:

```text
Prism diff review remains unchanged
        +
Opt-in repo-wide CPG navigation
        +
Deterministic hierarchical localization
        +
Optional SCIP/Glean precision backend
```

This keeps the current value of Prism intact while moving it toward the strongest evidence-backed LLM codebase understanding stack.
