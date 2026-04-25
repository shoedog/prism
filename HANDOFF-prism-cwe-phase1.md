# Phase 1 Implementation Handoff — CWE-78/22 Go

**Date:** 2026-04-25
**From:** Phase 1 design + plan session
**To:** Fresh implementation session
**Status:** Ready to execute. Architecture locked, plan written, sub-agent dispatch is the next step.

---

## 1. State on main

| Artifact | Path | SHA |
|---|---|---|
| Phase 0 hygiene + ALGORITHMS.md | (PR #71 merged) | `9d756a5` |
| Eval-team CWE coverage handoff | `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md` | (read-only, 2026-04-22) |
| Prism ACK | `ACK-prism-cwe-coverage-handoff.md` | `df04a65` |
| Eval-team go signal (Phase 1 cleared) | `~/code/agent-eval/analysis/RE-prism-cwe-coverage-handoff-go-signal.md` | (read-only) |
| Phase 1 design spec | `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` | `57ce096` |
| Phase 1 implementation plan | `docs/superpowers/plans/2026-04-25-phase1-cwe-go.md` | `83c7336` |

Working tree clean on `main`. Branch `claude/cwe-phase1-go` does not yet exist.

## 2. Read order (orientation)

Total reading time: ~15 minutes.

1. **`HANDOFF-prism-cwe-phase1.md`** — this doc, top-to-bottom (you're here).
2. **`docs/superpowers/plans/2026-04-25-phase1-cwe-go.md`** Pre-flight section + File Structure table — your authoritative execution guide. Skim Tasks 1, 11, 18 to feel the granularity.
3. **`docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`** §1 (deliverables) and §4.3 (risk table). The full spec is 595 lines — read on demand per-task, not upfront.
4. **`ACK-prism-cwe-coverage-handoff.md`** §1 (acceptance criteria) — what "shipped" vs "accepted" mean.
5. **`CLAUDE.md`** — project conventions (build commands, file organization, the **3-copy `coverage_test.rs` rule** that's load-bearing for tasks 9, 17, 27).

The three review files (`docs/superpowers/specs/2026-04-25-phase1-cwe-go-design-review{,-2,-3}.md`) document the design iteration history. Read only if you want the rationale behind a specific design decision.

## 3. Entry point

```bash
cd /Users/wesleyjinks/code/slicing
git checkout main
git pull
git checkout -b claude/cwe-phase1-go
```

Then invoke the subagent-driven-development skill with the plan as input:

```
Skill: superpowers:subagent-driven-development
args: "Execute the plan at docs/superpowers/plans/2026-04-25-phase1-cwe-go.md
       on branch claude/cwe-phase1-go. Three sequential commits; one
       sub-agent per commit; two-stage review (spec compliance + code
       quality) between commits. Plan is self-contained per commit."
```

Same scaffolding as Phase 0 (PR #71). The plan's Pre-flight P1–P4 verify the starting state; Tasks 1–9 are Commit 1; Tasks 10–17 are Commit 2; Tasks 18–28 are Commit 3; F1–F2 close out with push + PR.

## 4. Locked architectural commitments

These were settled during brainstorming + 3 review iterations. **Do not redesign in implementation.** If implementation friction surfaces, file `~/code/slicing/QUESTIONS-prism-cwe-phase1.md` per ACK §6 instead of re-deciding silently.

- **Lazy `OnceCell` + pull model** for framework detection (spec §2.5, §2.8).
- **First-match wins, ordered `[gin, gorilla_mux, nethttp]`** — router frameworks before fallback (spec §2.3).
- **Two storage locations for sinks:** cross-cutting in `taint.rs` consts (`GO_CWE78_SINKS`, `GO_CWE22_SINKS`); framework-gated in `FrameworkSpec.sinks` (spec §3.1).
- **`FlowPath` augmentation** in `data_flow.rs` (not a new `TaintValue` wrapper, not a side-table) for path-sensitive `cleansed_for` tracking (spec §3.5).
- **Textual co-occurrence heuristic** for `paired_check` recognizers; CFG-aware refinement deferred to Phase 1.5+ (spec §3.4, §3.8).
- **Shell cleanser dropped** as no-op in Phase 1; `SHELL_RECOGNIZERS = &[]` (spec §3.9).
- **Boolean `cleansed_for: BTreeSet<SanitizerCategory>`** — no confidence values (spec §3.6, ACK §3 Q3).
- **Quiet-mode default** when no framework matches (spec §3.4 / ACK §3 Q5).

## 5. Known limitations to surface (do not silently fix)

These are intentional false-negatives the eval team has accepted. The implementer should **not** silently improve them — if the test suite hits one, document it and move on:

- Path-validation paired-check direction ambiguity — both correct and inverted guard shapes suppress (spec §3.8).
- `syscall.Exec` slice-element taint = whole-slice DFG-conservative; no per-element tracking (spec §3.2).
- `HARDCODED_SECRET` keyword-prefixed LHS forms not matched (spec §1; ACK §4.1; deferred to Phase 2/3 registry redesign).
- PowerShell + exotic shell paths not in CWE-78 sink list (spec §3.2).

If C1 fixtures from the eval team end up exercising any of these and the eval team flags it post-merge, address as a Phase 1.5 follow-up — not as a Phase 1 amendment.

## 6. Acceptance criteria (ACK §1)

| # | Criterion | Validation |
|---|---|---|
| 1 | Taint fires on at least one CWE-78 + CWE-22 example in C1 fixtures | Post-merge by eval team; in-tree synthetic tests are the proxy |
| 2 | ≥80% sanitizer suppression rate on sanitizer test suite | In-tree, pinned by `integration_cwe_phase1_suppression` (Task 26) |
| 3 | Framework detection without per-run config | In-tree, pinned by `frameworks_*` tests (Tasks 7-8) |
| 4 | D2 coexistence — no Prism-side dedup | Behavioral; verified by absence of dedup logic |
| 5 | No Tier-1 regression — existing 1,406 tests pass | CI catches; Pre-flight P2 sets the baseline |

"Shipped" = PR merges. "Accepted" = eval team confirms criteria 1 + 5 via a `RE-*.md` reply doc.

## 7. Context budget

Phase 0 sub-agent execution (same shape, similar size) consumed ~70-80% of a 1M-context Opus session. Phase 1 plan adds ~30% more code than Phase 0 (framework infra + sanitizer registry + 10+10 fixture suite vs. 3 algorithms + 32 tests).

**Plan to land Phase 1 in similar budget**, but watch for these compounding pressures:
- Each sub-agent dispatch returns a multi-thousand-token report into the main thread.
- Reviewer (`superpowers:code-reviewer`) sub-agents tend to run long (one Phase 0 reviewer used 41 tool calls, 590s, looking for context).
- Iteration loops on review feedback amplify the above.

If the main thread hits ~80% context, file an interim status note and split the remaining commits across sessions. Each commit's task block is self-contained per the plan, so a mid-stream session split is low-friction.

## 8. Post-merge

When the PR merges:

1. File `~/code/slicing/STATUS-prism-cwe-phase1.md` summarizing what shipped, which acceptance criteria are pinned by which tests, and any deferred items rolled to Phase 1.5.
2. Notify the eval team via the `RE-*.md` reply convention. Reply doc convention is established in `~/code/agent-eval/analysis/RE-prism-cwe-coverage-handoff-go-signal.md` ("Reply docs from us: file under `~/code/agent-eval/analysis/RE-*.md` (this doc establishes the pattern)").
3. Eval team validates against `~/code/agent-eval/cache/prism-cwe-fixtures/` + 2-3 Tier 1 fixtures. Their reply lands at `~/code/agent-eval/analysis/RE-*.md`.
4. Phase 2 (Python) starts on their next go-signal. Same shape: brainstorm → spec → plan → execution.

---

*Hand-off complete. Open the plan, run Pre-flight P1, and dispatch the Commit 1 implementer sub-agent.*
