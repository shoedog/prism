# Documentation Organization Cleanup Plan

Status: Revised after Claude/Codex plan review
Date: 2026-07-01

## Objective

Finish the second documentation cleanup pass after the initial feature/archive
split. The target is a documentation tree where active implementation direction
is easy to find, historical review material remains auditable, and root
`docs/` does not become a mixed holding area again.

This pass covers two remaining areas:

1. `docs/prism-query-layer/`, which now acts as a legacy backlog and research
   holding area.
2. Root-level `docs/*.md`, which still mixes canonical references, language
   expansion plans, evaluation notes, stale worktree notes, and older analysis.

## Review Findings Folded

Claude and Codex both found that the original plan under-specified
compatibility. Moving the remaining docs without a repo-wide stale-path rewrite
would break source comments, coverage metadata, and `docs/superpowers/` deep
links. This revision folds those findings by making exact-path compatibility a
release gate.

Accepted changes:

- Rewrite exact path references across `src/`, `docs/`, `coverage/`, `eval/`,
  scripts, skills, and root project docs when a file moves.
- Do not rely on a directory README redirect to preserve file-level deep links.
- Do not create a new top-level query-layer roadmap in this pass. Keep the
  existing `docs/features/query-layer/navigation/follow-on-roadmap-2026-06-23.md`
  as the current query-layer roadmap unless a future feature slice deliberately
  replaces it.
- Promote `E12-type-system.md` to a current type-system feature doc because
  source module docs point to it as the full design.
- Keep root `docs/MCP.md` stable as the agent-facing setup guide.
- Treat coverage trackers as source-of-truth docs; moving them requires updating
  `coverage/matrix.json` and maintainer how-to references in the same change.

## Recommendation

Do not keep `docs/prism-query-layer/` as a permanent legacy bucket. Treat it as
a temporary triage queue. Its target state should be either empty except for a
README redirect/status map, or removed after all retained content has been
promoted or archived.

Apply the same rule to root `docs/*.md`: root `docs/` should be a navigation
layer and home for truly cross-cutting stable references, not a default location
for plans, reviews, or feature-specific analysis.

## Routing Rules

| Document class | Target |
|----------------|--------|
| Current feature spec, design, plan, status, or roadmap | `docs/features/<feature>/...` |
| Query-layer navigation, evidence, MCP refresh, or import-resolution content | `docs/features/query-layer/<area>/...` |
| Query-layer multi-repo serving or HTTP transport planning | `docs/features/query-layer/mcp-serving/...` |
| CPG architecture or implementation status | `docs/features/cpg/...` |
| Language coverage or parser/resolution expansion plans | `docs/features/language-coverage/...` unless a narrower feature exists |
| Maintainer procedures | `docs/how-to/...` |
| Tier-A baselines, evaluation run artifacts, and evaluation methodology | `docs/eval/...` |
| Historical review, spec-review, plan-review, code-review, transcript, or provenance record | `docs/archive/review-artifacts/...` |
| Superseded plans and analysis retained for context | `docs/archive/plans/...` or `docs/archive/analysis/...` |
| Repo-wide stable references | root `docs/`, linked from `docs/README.md` |
| Agent setup and MCP user guide | root `docs/MCP.md` |

## Status Labels

Use these labels in routing READMEs and inventory tables:

- **Current:** authoritative planning or implementation direction.
- **Backlog:** still plausible follow-up work, but not authoritative design.
- **Historical:** useful context for completed or superseded work.
- **Archived:** retained for auditability; not current direction.
- **Redirect:** compatibility entry point that points to canonical locations.

## Proposed Query-Layer Classification

| Current path | Proposed action | Target |
|--------------|-----------------|--------|
| `docs/prism-query-layer/analysis-prism-extension.md` | Archive as historical architecture analysis. | `docs/archive/analysis/prism-query-layer/` |
| `docs/prism-query-layer/research-llm-codebase-navigation.md` | Archive as research provenance; update any current references. | `docs/archive/analysis/prism-query-layer/` |
| `docs/prism-query-layer/substrate-analysis-MCP-2026-06-09.md` | Archive as historical substrate analysis. | `docs/archive/analysis/prism-query-layer/` |
| `docs/prism-query-layer/substrate-analysis-baseline-noMCP-2026-06-09.md` | Archive as historical baseline analysis. | `docs/archive/analysis/prism-query-layer/` |
| `docs/prism-query-layer/substrate-deepen-widen-harden-brief.md` | Archive original brief; do not hand-consolidate in this pass. | `docs/archive/analysis/prism-query-layer/` |
| `docs/prism-query-layer/plan3a-cache-followups.md` | Move verbatim as historical backlog; update exact references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/plan3b-module-map-followups.md` | Move verbatim as historical backlog; update exact references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/plan3c-mcp-followups.md` | Move verbatim as historical backlog; update exact references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/planA-followups.md` | Move verbatim as historical backlog; update exact references in `src/` and docs. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/s1-followups.md` | Move verbatim as historical backlog; update exact references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/tier-a-followups.md` | Move verbatim as historical backlog; update exact references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/tier-a-handoff-2026-06-12.md` | Archive as historical handoff. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/phase-ip-pr1-review-followups-2026-06-15.md` | Move verbatim as historical backlog; update exact references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/tier2-planA-taint-reaches-constraints.md` | Archive as historical constraint note; update any current taint references. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/tier2-planB-taint-reaches-constraints-merged-2026-06-10.md` | Archive as historical constraint note. | `docs/archive/plans/prism-query-layer/` |
| `docs/prism-query-layer/README.md` | Keep as redirect/status map during transition. | `docs/prism-query-layer/README.md` |

## Proposed Root And Stray Top-Level Doc Classification

| Current path | Proposed action | Target |
|--------------|-----------------|--------|
| `docs/README.md` | Keep root index; update after moves. | root `docs/README.md` |
| `docs/MCP.md` | Keep root as the stable agent-facing MCP setup and user guide. | root `docs/MCP.md` |
| `docs/SPEC-ALGO-LANGUAGE-COVERAGE.md` | Promote to language coverage feature docs. | `docs/features/language-coverage/spec-algo-language-coverage.md` |
| `docs/cross-language-coverage.md` | Promote to language coverage feature docs. | `docs/features/language-coverage/cross-language-coverage.md` |
| `docs/language-expansion-plan.md` | Promote to language coverage feature docs. | `docs/features/language-coverage/language-expansion-plan.md` |
| `docs/language-analysis-gaps.md` | Promote to language coverage feature docs. | `docs/features/language-coverage/language-analysis-gaps.md` |
| `docs/jsx-tsx-react-plan.md` | Promote under language coverage. | `docs/features/language-coverage/jsx-tsx-react-plan.md` |
| `docs/shell-bash-plan.md` | Promote under language coverage. | `docs/features/language-coverage/shell-bash-plan.md` |
| `docs/terraform-hcl-plan.md` | Promote under language coverage. | `docs/features/language-coverage/terraform-hcl-plan.md` |
| `docs/prism-ccpp-expansion-plan.md` | Promote under language coverage. | `docs/features/language-coverage/c-cpp/expansion-plan.md` |
| `docs/prism-ccpp-gap-analysis.md` | Promote under language coverage. | `docs/features/language-coverage/c-cpp/gap-analysis.md` |
| `docs/c-cpp/function-pointer-resolution.md` | Move with C/C++ coverage docs or link from the new index. | `docs/features/language-coverage/c-cpp/function-pointer-resolution.md` |
| `docs/arrow-anon-functions.md` | Archive as historical analysis with status note; tests show the core behavior landed. | `docs/archive/analysis/language-coverage/` |
| `docs/access-network-analysis-evaluation.md` | Archive as historical language/domain expansion analysis. | `docs/archive/analysis/language-coverage/access-network-analysis-evaluation.md` |
| `docs/mcp-http-multi-repo-evaluation.md` | Promote as active query-layer MCP serving input; keep `docs/MCP.md` as the root user guide. | `docs/features/query-layer/mcp-serving/mcp-http-multi-repo-evaluation.md` |
| `docs/eval/wp2-timing.md` | Keep under evaluation docs. | `docs/eval/wp2-timing.md` |
| `docs/ruff-typepath-recovery-roadmap-2026-06-21.md` | Archive as historical precision-recovery roadmap; update superpowers references. | `docs/archive/plans/prism/ruff-typepath-recovery-roadmap-2026-06-21.md` |
| `docs/prism-meta-analysis-2026-06-10.md` | Archive as historical Prism analysis; update current roadmap references. | `docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md` |
| `docs/prism-follow-up-e13-next.md` | Archive as historical follow-up note. | `docs/archive/plans/prism/prism-follow-up-e13-next.md` |
| `docs/E12-type-system.md` | Promote to current type-system feature docs. | `docs/features/type-system/e12-type-system.md` |
| `docs/owner-key-identity-analysis-2026-06-20.md` | Archive as historical Prism analysis; update companion-analysis references. | `docs/archive/analysis/prism/owner-key-identity-analysis-2026-06-20.md` |
| `docs/parallel-brief-r6-taint-split-2026-06-17.md` | Archive as historical implementation brief. | `docs/archive/plans/prism/parallel-brief-r6-taint-split-2026-06-17.md` |
| `docs/parallel-brief-s4-scope-honesty-2026-06-17.md` | Archive as historical implementation brief. | `docs/archive/plans/prism/parallel-brief-s4-scope-honesty-2026-06-17.md` |
| `docs/stale-worktree-salvage-notes-2026-05-11.md` | Archive as operational incident note. | `docs/archive/operations/stale-worktree-salvage-notes-2026-05-11.md` |

## Implementation Phases

### Phase 1: Add Governance and Inventory

- Add a documentation feature area with this plan and a README.
- Add a machine-readable or markdown inventory table for remaining root and
  query-layer docs.
- Add `docs/archive/analysis/`, `docs/archive/plans/`, and
  `docs/archive/operations/` indexes before moving files there.

### Phase 2: Move Obvious Historical Material

- Move query-layer historical analyses, backlog notes, and handoffs into archive
  directories.
- Move root historical briefs, stale worktree notes, and superseded analysis
  into archive directories.
- Preserve content verbatim except for optional status banners and link updates.

### Phase 3: Promote Current Feature Material

- Create `docs/features/language-coverage/` and move language expansion plans
  there.
- Create `docs/features/type-system/` and promote `E12-type-system.md`.
- Create `docs/features/query-layer/mcp-serving/` for multi-repo serving and
  HTTP transport planning.
- Keep the existing query-layer navigation roadmap authoritative. Do not create
  `docs/features/query-layer/roadmap.md` in this pass.

### Phase 4: Update Routing and Validate

- Update `docs/README.md`, `docs/features/README.md`,
  `docs/features/query-layer/README.md`, `docs/how-to/update-coverage-matrix.md`,
  `coverage/matrix.json`, and archive READMEs.
- Keep `docs/prism-query-layer/README.md` as a redirect during transition.
- Replace links to moved paths.
- Run the validation commands below and require zero stale references for moved
  exact paths.

## Required Validation

Run these checks before review:

```bash
git diff --check origin/main...
rg -n --glob '!docs/archive/**' "docs/prism-query-layer/(analysis-prism-extension|research-llm-codebase-navigation|substrate-analysis-MCP-2026-06-09|substrate-analysis-baseline-noMCP-2026-06-09|substrate-deepen-widen-harden-brief|plan3a-cache-followups|plan3b-module-map-followups|plan3c-mcp-followups|planA-followups|s1-followups|tier-a-followups|tier-a-handoff-2026-06-12|phase-ip-pr1-review-followups-2026-06-15|tier2-planA-taint-reaches-constraints|tier2-planB-taint-reaches-constraints-merged-2026-06-10)\\.md"
rg -n --glob '!docs/archive/**' "docs/(SPEC-ALGO-LANGUAGE-COVERAGE|cross-language-coverage|language-expansion-plan|language-analysis-gaps|jsx-tsx-react-plan|shell-bash-plan|terraform-hcl-plan|prism-ccpp-expansion-plan|prism-ccpp-gap-analysis|arrow-anon-functions|access-network-analysis-evaluation|mcp-http-multi-repo-evaluation|ruff-typepath-recovery-roadmap-2026-06-21|prism-meta-analysis-2026-06-10|prism-follow-up-e13-next|E12-type-system|owner-key-identity-analysis-2026-06-20|parallel-brief-r6-taint-split-2026-06-17|parallel-brief-s4-scope-honesty-2026-06-17|stale-worktree-salvage-notes-2026-05-11)\\.md"
rg -n --glob '!docs/archive/**' "docs/c-cpp/function-pointer-resolution\\.md"
rg -n -P --glob '!docs/archive/**' "(?<![\\w/.-])(analysis-prism-extension|research-llm-codebase-navigation|substrate-analysis-MCP-2026-06-09|substrate-analysis-baseline-noMCP-2026-06-09|substrate-deepen-widen-harden-brief|plan3a-cache-followups|plan3b-module-map-followups|plan3c-mcp-followups|planA-followups|s1-followups|tier-a-followups|tier-a-handoff-2026-06-12|phase-ip-pr1-review-followups-2026-06-15|tier2-planA-taint-reaches-constraints|tier2-planB-taint-reaches-constraints-merged-2026-06-10|SPEC-ALGO-LANGUAGE-COVERAGE|cross-language-coverage|language-expansion-plan|language-analysis-gaps|jsx-tsx-react-plan|shell-bash-plan|terraform-hcl-plan|prism-ccpp-expansion-plan|prism-ccpp-gap-analysis|arrow-anon-functions|access-network-analysis-evaluation|mcp-http-multi-repo-evaluation|ruff-typepath-recovery-roadmap-2026-06-21|prism-meta-analysis-2026-06-10|prism-follow-up-e13-next|E12-type-system|owner-key-identity-analysis-2026-06-20|parallel-brief-r6-taint-split-2026-06-17|parallel-brief-s4-scope-honesty-2026-06-17|stale-worktree-salvage-notes-2026-05-11|function-pointer-resolution)\\.md\\b"
```

The stale-reference scans intentionally exclude `docs/archive/**` so historical
archive bodies can preserve original provenance text. They should return only
deliberate compatibility text in routing READMEs, the move inventory, or this
plan. All source comments, coverage JSON, how-to docs, eval docs, and
superpowers deep links must point to the new canonical or archive paths. The
basename scan covers prose references such as `s1-followups.md` that do not
include a `docs/...` prefix while avoiding matches inside already-updated full
paths.

## Guardrails

- Prefer `git mv` for all moves so review can distinguish moves from rewrites.
- Do not delete retained docs in this pass; archive first.
- Do not rewrite archived transcripts or review artifacts except for index
  updates or an archive status banner.
- Do not treat old pre-`CpgContext` speed measurements as current performance
  targets. Keep them only as historical context.
- Keep active docs concise: one final spec/plan/status per feature or slice, with
  historical review iterations routed to archive.
- Do not rely on directory-level redirects for moved files. Either update every
  exact-path reference or leave an intentional per-file compatibility stub.

## Review Questions

External reviewers should focus on:

1. Whether the routing taxonomy has ambiguous or missing buckets.
2. Whether any proposed archive target should instead be a current feature doc.
3. Whether every moved exact-path reference is covered by the validation gate.
4. Whether any per-file compatibility stubs are required after repo-wide rewrites.
5. Whether the validation plan is sufficient for a docs-only reorganization.
