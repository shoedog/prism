# Prism Query-Layer Follow-on Roadmap

Status: Historical roadmap with implementation status notes; updated 2026-07-01
Date: 2026-06-23
Branch: `evidence-view-spec`

> Current status: several items in this roadmap have since landed. EvidenceView contract hardening, production/test
> evidence labels, and MCP auto-refresh policy plumbing are implemented in the current codebase. Multi-repo stdio
> and Tier-C agent-value work were not refreshed by this sweep and should be revalidated before implementation.

## Recommendation

The best immediate follow-on is the EvidenceView contract hardening slice. It builds directly on the MCP
work, improves LLM usefulness without changing resolver or CPG behavior, and has a low conflict surface:
mostly `src/mcp/input.rs`, `src/mcp/output.rs`, `src/mcp/tools.rs`, `src/mcp/evidence_view.rs`, and MCP
tests.

The first implementation slice should stay opt-in profile/view work only. Canonical `Evidence` remains the
stable source of truth and default MCP output.

## Five-item roadmap

### 1. EvidenceView contract hardening

Purpose: make `agent_json` and `agent_markdown` strong LLM navigation packets, not just alternate renderings.

Scope:

- Add normalized view locations beside display `loc` strings.
- Add canonical `symbol_ref` handles where present.
- Add deterministic reason arrays.
- Add group summaries and representative locations.
- Add source locations to next-query hints.
- Keep all behavior opt-in through `format: "agent_json"` or `format: "agent_markdown"`.

Conflict surface:

- `src/mcp/evidence_view.rs`
- `src/mcp/input.rs`
- `src/mcp/tools.rs`
- `src/mcp/output.rs`, only if a helper is needed
- MCP tests

References:

- `docs/superpowers/specs/2026-06-23-prism-evidence-view-contract-design.md`
- `docs/features/query-layer/evidence/llm-evidence-shaping-design-2026-06-23.md`
- `docs/features/query-layer/evidence/llm-evidence-shaping-implementation-plan-2026-06-23.md`
- `docs/features/query-layer/evidence/evidence-profile-quality-plan-2026-06-23.md`
- `docs/features/query-layer/evidence/evidence-profile-quality-implementation-handoff-2026-06-23.md`
- `src/mcp/evidence_view.rs`

### 2. Indexing policy: production/test labels

Purpose: improve answer precision by separating production evidence from test evidence without hiding tests.

Scope:

- Respect `.gitignore` and add `.prismignore`.
- Add `is_test` labels using path/language heuristics.
- Add a `scope: prod|test|all` query parameter after labels exist.
- Keep tests indexed by default; label rather than exclude.

Why it matters:

- "2 production callers, 13 test callers" is a materially better LLM answer than an unlabeled list of 15
  callers.
- This improves precision without changing call-resolution semantics.

References:

- `docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md` section 7, "Indexing policy - tests, fixtures, vendored and
  generated code"
- `src/repo_loader.rs`
- `src/navigation/types.rs`
- `src/mcp/input.rs`

### 3. MCP multi-repo over stdio

Purpose: let one Prism MCP process answer queries for multiple repos, selected per request, before adding
HTTP transport.

Scope:

- Add an optional per-tool-call `repo` argument.
- Resolve repo roots through a bounded repo-to-session cache.
- Add allowlist/root-prefix safety and canonicalization.
- Keep stdio transport first; defer HTTP.

Why it matters:

- It directly unblocks multi-repo agent workflows and a2a-bridge serve-mode use.
- It is smaller than HTTP because it avoids new async server dependencies.

References:

- `docs/features/query-layer/mcp-serving/mcp-http-multi-repo-evaluation.md`, "Ask 2 - multi-repo serving"
- `src/mcp/session.rs`
- `src/mcp/transport.rs`
- `src/mcp/input.rs`
- `src/mcp/tools.rs`

### 4. Tier-C agent value A/B

Purpose: measure whether the navigation and EvidenceView work improves real agent outcomes, not only unit
accuracy.

Scope:

- Run code-review-benchmark or equivalent tasks with Prism navigation available vs unavailable.
- Track task success, false claims, useful citations, extra turns, and token cost.
- Use EvidenceView profiles as experimental variables once the contract hardening slice lands.

Why it matters:

- Tier-A measures call-resolution accuracy. Tier-C answers whether agents actually make better decisions with
  the navigation layer.

References:

- `docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md` section 8, "Priority synthesis across all seven"
- `docs/eval/tier-a/`
- `eval/README.md`

### 5. Auto-refresh on drift

Purpose: close the edit-then-query stale-index failure mode for long-lived MCP sessions.

Scope:

- Keep current staleness warning behavior.
- Add debounced auto-refresh only after the incremental rebuild defect is fixed.
- Preserve explicit snapshot/freshness metadata so agents can tell when answers reflect current files.

Why it matters:

- MCP servers can live for hours. A model that edits a file and then queries stale navigation evidence can make
  confident wrong decisions even if the underlying CPG is accurate.

References:

- `docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md` section 4, "Mid-session staleness"
- `src/mcp/freshness.rs`
- `src/mcp/session.rs`
- `src/mcp/transport.rs`
- `src/cpg/build.rs`

## Sequencing

Recommended order:

1. EvidenceView contract hardening.
2. Indexing policy labels.
3. MCP multi-repo over stdio.
4. Tier-C A/B once the LLM-facing packet is stable.
5. Auto-refresh after incremental rebuild correctness is addressed.

EvidenceView is first because it is high leverage, low conflict, and directly improves the value of the MCP
surface already merged. It also gives Tier-C a better treatment condition to measure.
