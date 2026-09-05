# Handoff — navigation onboarding report v1

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing` · `main`
**Exact merged state:** PR #235 merge `a531355420c47948a415fb055fe7c82b13210252`
**Current root custody:** PR #236 merge `fd4ddf05fa0f7677708d086b7de5ec1d327c2482`

## 0. Current verdict

**MERGED (#235), DOCUMENTED, REVIEWED, AND VERIFIED.** The successor product increment
is a CLI-only, bounded onboarding report.

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
- RED CLI target: the initial 3/3 tests, then the added cache contract, all failed at
  Clap's unknown `onboard` subcommand;
  repository loading and output-file creation were not reached.
- Focused GREEN is 12/12: 3 report integration tests, 6 CLI tests, and 3 internal
  telemetry/graph/rendering tests. The cached CLI path creates both the CPG cache and
  resolved-call-edge sidecar. Existing-output and missing-parent errors emit no report
  to stdout; existing bytes are preserved.
- Review round 1 found one WRONG (backslash does not escape an embedded backtick inside
  a CommonMark code span) and one SMELL (a failed write could leave a partial new
  report). The stronger Markdown assertion failed RED, then dynamic fences passed;
  write/sync failure now attempts cleanup and reports cleanup failure explicitly.
- Review round 2 found one WRONG: documented `--repo .` emitted project `.` rather than
  the repository basename. A cwd-relative regression failed RED and passes after
  canonical-root naming. Findings converged 2 to 1, so the declared cap received a
  disclosed final confirmation pass; it found 0 WRONG and 0 SMELL.
- Format, base-to-candidate diff check, all-target MCP check, and configured all-target
  MCP Clippy are green. Clippy retains the repository's existing non-fatal warnings.
- Full default suite is green: 3,706 total = 3,705 passed + 1 ignored, 0 failed.
- Full MCP suite is green: 3,896 total = 3,895 passed + 1 ignored, 0 failed.
- All five required GitHub checks passed on exact head `e6b46b6`: Format Check, Clippy
  Lint, Test Suite, Language Coverage Matrix, and Coverage.
- Each Tier-A invocation had an immediately preceding release build. Matrix-only is
  green: 104/104 cases `ok`. Quick completed with 0.000 oracle/SUT error rates, a
  quiescent oracle, and clean corpus/SUT, but exited 2 solely because current corpus
  `bff9765139e4` differs from pinned `20c8490591a3`; no baseline was changed. The
  inherited `target-c-method` flip candidate and two expected pinned missing outcomes
  remain; there are no matrix regressions.
- LSP semantic navigation is unavailable; structural Prism navigation plus direct
  source reads supplies blast-radius evidence.

## 4. Custody

- Root `main` was rebound to exact PR #234 merge `90c522b` before the implementation
  branch, then fast-forwarded after the server-side merge to exact PR #235 merge
  `a531355420c47948a415fb055fe7c82b13210252`.
- Design/roadmap/handoff checkpoint is `393e0cf`; primary RED is `dbfb890`; cache RED
  extension is `1c8e558`; focused-GREEN implementation is `cd8c1fc`; review hardening
  is `baae280`; diff-gate cleanup is `bcbc118`; full-suite custody is `bff9765`;
  Tier-A verification refresh and exact PR head are `e6b46b6`. GitHub records PR #235
  merged at 2026-09-04T23:10:06Z as merge commit `a531355`.
- Root's pre-existing untracked `.superpowers/` and
  `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
