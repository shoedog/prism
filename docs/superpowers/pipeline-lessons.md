# Pipeline lessons — multi-agent P-item execution (rounds 1–5, 2026-07-02 → 2026-07-03)

Durable lessons from executing `docs/analysis/prism-llm-and-accuracy-plan.md` items P1–P10
(PRs #149–#160) via the owner-approved pipeline: codex gpt-5.5 xhigh spec review →
Sonnet implementers in isolated worktrees (TDD) → Opus task review ∥ codex impl review →
fix waves → codex re-review of fix deltas → owner merge. Update this file at each round
close-out; the per-round detail lives in the plan doc's status blocks and
`docs/superpowers/specs/*-spec.md` SHIPPED headers.

## Process lessons

1. **Re-review the fix delta, always.** Streak through round 5: **seven consecutive**
   codex re-reviews of fix waves caught a real defect in the fix itself (P3, P5, P7,
   P6bc, P8, P10, P9×2). The defects live at the *fix×interaction* layer — the fix is
   locally correct but breaks caching, incremental rebuilds, or an adjacent behavior.
   Exception that survives review: purely mechanical prescribed fixes may take
   controller hunk-verification instead of a full codex round.

2. **Restructures regress orthogonally.** Round 5's sharpest instance: a P9 fix wave
   restructured candidate extraction to fix a shadow-detection MAJOR and silently
   regressed *caller attribution* — behavior orthogonal to the finding being fixed
   (anonymous wrappers fell back to `<module>`). Only the re-re-review caught it.
   **Adjustment:** when a fix wave restructures (moves/reshapes code rather than
   patching in place), the re-review prompt must explicitly ask "what behavior did the
   restructure carry, and does it still hold?" — not just "is the finding fixed?"

3. **Ground the plan's cited mechanism before writing the brief.** The plan cited
   `FlowPath.cleansed_for` reuse for P10; grounding showed `taint_reaches` never builds
   FlowPaths and the naive design would have minted false negatives. A cheap
   grounding-Explore pass before the brief caught it; the controller redesign then held
   through five codex passes. Plans rot; briefs must be grounded against HEAD.

4. **Spec-review the brief before implementation.** Every round's batched codex spec
   review found at least one BLOCKER/MAJOR-class flaw in the brief (P10's node-presence
   proof, P9's non-navigable pseudo-caller, P5's CallSite-pipeline hole, P8's wildcard
   poison). Two hours of review is cheaper than a wrong implementation.

5. **Two independent reviewers see different things.** Opus task reviews verify spec
   compliance and catch report/diff mismatches; codex impl reviews find semantic
   soundness holes (every round: real defects all claude-family gates missed). Neither
   subsumes the other. Where they conflict (P9's `app.use` gap: opus "acceptable",
   codex "fix"), the controller adjudicates on the merits — precedence goes to the
   argument, not the model.

6. **Line anchors rot; symbols don't.** Briefs written against a moving main should
   name symbols to grep (`property_accesses` → every plumbing site) rather than
   line numbers, or mark anchors as hints. Post-#158 anchors were stale within one
   round.

7. **Never rebase under a running agent; never re-bump caches for pre-merge fixes.**
   Rebase after the fix wave completes. A cache version bumped once per PR covers all
   its fix waves — the shipped transition is one step.

8. **Ledger + handoff before compaction.** The git-ignored
   `.superpowers/sdd/progress.md` ledger records every verdict/dispatch; the handoff
   doc carries the cold-start map. Both saved this execution across multiple
   compactions with zero re-dispatched work.

## Engineering doctrines (bind reviewers and implementers)

1. **Consumer-visibility doctrine** (rounds 2–5, codex-ruled): uncertainty tiers need
   consumer-visibility tiers. Name-coincidence candidates = nav-only. Registration-
   grounded candidates reach non-nav consumers only at exactly one target.
   **Entrypoint facts** (P9 framework registrations) are nav-only regardless of target
   count — a registration is not an invocation; feeding it to taint/slice would assert
   dataflow that doesn't exist at the registration line. Nothing below Exact feeds an
   asserted finding.

2. **Prove structure by byte-span reconstruction, never by graph edge labels** (P10).
   Derived graphs over-approximate (first-parent BFS trees, same-line
   `AssignmentPropagation` from a use to *every* same-line def). Any claim of the form
   "this edge is a real X" must be re-proven from the AST via byte spans
   (`descendant_for_byte_range` + parent walk), not trusted from the edge kind.

3. **Index-routed scope checks are blind to anonymous scopes** (P9). Anything keyed by
   `FunctionId` (locals indices, enclosing-function lookups) structurally cannot see
   anonymous arrows/IIFEs. Shadow/binding decisions must walk the call node's actual
   AST ancestors. Conversely, *attribution* decisions (who is the caller) may want to
   skip anonymous nodes — the two walks answer different questions; don't merge them.

4. **Name inference ≠ referenceability** (P9). The function-name inference layer names
   object-property arrows, member assignments (`exports.h = ...`), and class members —
   none of which a bare identifier can reference. Any bare-identifier resolution must
   filter by definition-site AST shape, not by presence in the functions index.

5. **Repo-wide facts break per-file incremental caching** (P8). If extraction for file
   A depends on facts from file B (macro shadow sets, export maps), persist the fact on
   the graph and compare on incremental rebuild — mismatch → full rebuild. Cheap and
   sound.

6. **Second copies of resolution logic drift** (P4): `scoped_caller_site_match_count`
   independently re-derives R4c name-correlation — any R4c change must update it too.
   Grep for parallel implementations before changing a resolution rung.

7. **The safe failure direction is a design input.** Sanitizer verdicts: a missed
   `Sanitized` stays `Reached` (safe); a false `Sanitized` hides a vulnerability
   (unsafe) — so every ambiguity resolves toward `Reached` (paired-check exclusion,
   positional-only default, direct-call-RHS-only). Framework edges: under-recording is
   safe, false edges are not. State the direction in the brief; reviewers verify each
   ambiguity resolves toward it.

8. **Measurement noise is real** (P4): quick-M2 exact-tier deltas can be pure
   line-shift resampling noise when a branch adds lines to instrumented files.
   Diagnose with repeat-run + shared-probe byte-identity controls before re-baselining.
   Related: rescore, never re-run, when arm outputs are saved.

## Follow-up queue (durable, self-tracking where possible)

- Nested-test-module `use super::*` callers gap — `known_fail` fixture
  `eval/fixtures/rust/nested_test_module_glob_gap/` flips to `flip_candidate` when fixed.
- Advisory/CWE sanitizer recognizers still cross-match languages (P10 gated the verdict
  path only; deliberate).
- `prune_graph_to_reasoning` keeps forward-hanging leaves only for `SanitizedBy`; any
  future forward-hanging edge kind needs the same treatment (one-class fix in place).
- `--review-no-diagrams` (P1 residual: diagram payloads dominate compacted review output).
- Adoption-eval re-run after SKILL.md changes (owner-gated, live API cost).
