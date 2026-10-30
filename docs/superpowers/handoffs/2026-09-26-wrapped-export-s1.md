# Handoff — wrapped-export resolution S1 (PRs #325 plan, #326 impl): ready for owner merge; next S1b

**Written:** 2026-09-26 · **By:** Claude Code controller session fba3a940 · **Provider:** claude
**Workspace:** shoedog/prism · `feat/wrapped-export-s1` · **Measured state:** `[MEASURED]` HEAD `0d85e239` (pushed) · Tree CLEAN · Probe `git status --short; git log -1` · Output inline
**Predecessor:** `docs/superpowers/handoffs/2026-09-25-entry-call-proof.md`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` No implementer or reviewer is running; the last sol confirmation finished and was saved. — **RESOLVED 2026-09-26**
**(b) Custody exposure** — `[MEASURED]` All commits are pushed. Evidence lives under `~/prism-evidence/wrapped-export/`: `planning/`, `replan/`, `spec-reviews/`, `impl-s1/` and `acceptance/`, each with a `MANIFEST.sha256` where it was generated. Corpus F's identity is only in `planning/CORPORA-PRIVATE.txt`. — **RESOLVED 2026-09-26**
**(c) In flight / irreversible** — None.
**(d) Authorization granted but not exercised** — The owner merges PRs; the controller does not merge. Owner, 2026-09-25: "prioritize using opus-5.5 subagent for spec planning and implementation. with sol or astra for review. escalate to fable for planning on review rejection", and "Send implementation PR to opus-5.5, you can use subagent".

## 1. Resume order

1. The owner merges #325, then #326. After #325 merges, check `gh pr view 326 --json baseRefName`, retarget to `main` if needed, and confirm CI runs green on #326. Until then it reports no checks, because CI runs only on PRs into `main`.
2. Next lane, **S1b**, as SPEC §12 records it: span-verify all JS/TS export routes.
   - The D4 list/default `Local` false Exact (P3 C06/C21).
   - The R3 `ImportQualified` namespace decoy (C62, S1b-RED-1).
   - The R4 `LocalDef` producer-local non-JSX and multi-target case (C63, S1b-RED-2).
   - The lowercase-tag intrinsic sites on the plain `Local` route (Opus impl-r1 W1 alt (2), probe Z02b).

   Plan it with an Opus-5.5 subagent. Reviews go to sol and Opus-5.5 in parallel. On a rejection or an open-class cap, escalate to Fable.
3. Then plan tsconfig `paths` / workspace-package resolution. It is projected, not measured, at about 3.2k drops on X and 2.8k on F.

**STOP conditions:** a red CI on #326 after retargeting; the owner choosing a different next lane.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| #325 plan packet | pending owner merge | `[MEASURED]` CI green (Test Suite, Coverage, Format, Language Coverage Matrix) |
| #326 implementation | pending owner merge | `[MEASURED]` Opus-5.5 and sol both APPROVE the narrow confirmation (`spec-reviews/impl-confirm-*.md`) |
| Corpus acceptance | done | `[MEASURED]` X 107 (`265790a3…`), F 4 (`cc5ada8f…`), R 0, T 0; identical at `11fac44b` and `a270bf01` (`acceptance/`) |
| Budget | done | `[MEASURED]` src 342 / tests 660 / combined 1,002, against owner caps of 350 / 680 / 1,010 |
| Suites | done | `[INHERITED]` from the implementer logs, cross-checked by the reviewers: 4,583/0/1 default, 4,776/0/1 mcp, Tier-A 162/162, Node gate 851/0/3 |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `CLAUDE.md` (lands with #326) | Exact was undefined | `[MEASURED]` A static-binding paragraph is added: runtime mutation is out of model for every rung |
| memory `project_wrapped_export_resolution.md` | — | `[MEASURED]` Updated to ready-for-merge |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Merge #325 → #326 | pending | Owner merges; controller verifies the retarget and CI | owner | PR 325, 326 |
| 2 | S1b plan | not started | Opus planner, measure first | #1 | SPEC §12 |
| 3 | `paths` resolution plan | not started | After S1b | #2 | — |
| 4 | Clone cleanup | pending | Remove `~/code/prism-wrapped-export{,-impl,-review-sol,-review-opus}` after the merge | #1 | — |

## 5. Invariants and traps — do not do these

- Never publish corpus F names, paths or commits in this public repo; aggregates only. Tell reviewers never to open `fportal` files or `CORPORA-PRIVATE.txt`.
- Never hold a slice to a stricter Exact bar than the codebase applies. Probe base for the precedent and take the standard to the owner, as Branch P did.
- Never use `sed` with a `#` delimiter when the text contains `#NNN`; use Python replacement.
- Save subagent reviews to disk before pointing another agent at them. A reviewer's hand-back message is not a file.
- Never background `codex exec` with `&`, because it sends no completion notification. Use the Bash tool's `run_in_background`.
- Implementers skip early-stop checkpoints when they write tests in batches. Require a measurement after every batch.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base binary | `~/prism-evidence/wrapped-export/planning/bin/prism-base-12ca6e8e` (`60755871…`) |
| Final head binary | `~/prism-evidence/wrapped-export/acceptance/prism-head-a270bf01` (`98f7bcd1…`) |
| Plan branch / head | `plan/wrapped-export-resolution` @ `da1f5479` |
| Impl branch / head | `feat/wrapped-export-s1` @ `0d85e239` |
| Reviews | `~/prism-evidence/wrapped-export/spec-reviews/` (`r1-3-sol`, `impl-r1/r2-{sol,opus}`, `impl-confirm-{sol,opus}`) |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "S1 adds exactly the 107/4 audited Exact JSX edges and no static misbinding under the Branch-P model" · pass: INDEPENDENT (Opus-5.5 and sol, 2 rounds plus a confirmation, with base controls) · evidence tier: TEST-BACKED · record: `spec-reviews/impl-confirm-*.md`, `acceptance/`

**Questions the owner owes an answer to:**
1. Merge #325 and #326.
2. Confirm S1b as the next lane.
