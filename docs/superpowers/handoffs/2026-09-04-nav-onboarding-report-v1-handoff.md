# Handoff — navigation onboarding report v1

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex  
**Workspace:** `/Users/wesleyjinks/code/slicing` · `nav-onboarding-report-v1`  
**Exact base:** PR #234 merge `90c522b04ff16ebc076ce85a4f8df5f7f2da4f1f`

## 0. Current verdict

**RED CONTRACT ESTABLISHED; IMPLEMENTATION NEXT.** The successor product increment is
a CLI-only, bounded onboarding report. Production behavior does not exist yet.

## 1. Authority boundary

Implement `prism nav onboard` over one existing cached navigation session. Stdout is
default; optional output is explicit and create-new-only. No MCP write, source edit,
implicit project file, cache/schema transition, Java/LSP, or full-write tool is in
scope.

## 2. Evidence and plan

- `prism-nav` repo-map/caller evidence and direct source reads locate the seams at the
  CLI nav enum/dispatch, a new navigation report module, and output rendering.
- Existing `repo_map` and `call_stats` supply the module relation and resolution facts;
  the report does not change those projections or their MCP consumer.
- Design: `docs/superpowers/specs/2026-09-04-nav-onboarding-report-v1-design.md`.
- Plan: `docs/superpowers/plans/2026-09-04-nav-onboarding-report-v1.md`.
- Review cap: two self-review rounds.

## 3. Verification state

- RED navigation target: one compile error, the missing
  `prism::navigation::onboarding` module.
- RED CLI target: 3/3 selected tests failed at Clap's unknown `onboard` subcommand;
  repository loading and output-file creation were not reached.
- No GREEN tests yet.
- Full-suite and Tier-A gates remain pending.
- LSP semantic navigation is unavailable; structural Prism navigation plus direct
  source reads supplies blast-radius evidence.

## 4. Custody

- Root `main` was rebound to exact PR #234 merge `90c522b` before this branch.
- Design/roadmap/handoff checkpoint is `393e0cf`; the RED tests plus this refresh are
  the current commit candidate.
- Root's pre-existing untracked `.superpowers/` and
  `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
