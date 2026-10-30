# Handoff — post-#319 native positional-gap lane: bounded entry/call proof (PR #324) plus the stacked #322/#323

**Written:** 2026-09-25 · **By:** Claude Code controller session fba3a940 · **Provider:** claude
**Workspace:** shoedog/prism · `plan/bounded-entry-call-proof` · **Measured state:** `[MEASURED]` HEAD `60cbcba8` (pushed; the handoff commit follows) · Tree CLEAN · Probe `git status --short; git log origin/plan/bounded-entry-call-proof -1` · Output inline
**Predecessor:** `docs/superpowers/handoffs/2026-09-23-native-positional-gap-impl.md` (the observer lane, on #323)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` No implementer or reviewer process is running. The last one, sol task `b0np8ri06`, has completed. — **RESOLVED 2026-09-25**
**(b) Custody exposure** — `[MEASURED]` All plan and readout work is pushed (`60cbcba8`). The raw probe outputs (2 × 26 files) are single-copy under `~/prism-evidence/entry-call-proof/appendix-a/`, outside `/tmp`; their hashes are in `receipt.json`. — **RESOLVED 2026-09-25**
**(c) In flight / irreversible** — None.
**(d) Authorization granted but not exercised** — The owner merges PRs themselves ("pr 320 is merged", "sol approved -> PR 321 merged"). Do not merge #322, #323, or #324.

## 1. Resume order

1. Wait for the owner to merge. The order is #322 (gate natives) → #323 (observer, stacked on #322) → #324 (this readout, based on main; it cites #323's `docs/eval/native-positional-gap/`). After #322 merges, confirm #323 retargets to `main` (`gh pr view 323 --json baseRefName`).
2. Check CI on #324 (`gh pr checks 324`). It is docs only.
3. The next increment is the owner's choice. The measured candidate is **wrapped-export resolution**: `export const X = wrapper(arrow)` records no export (`src/ast.rs:2874-2890`), which causes 80 `UnknownName` JSX-use drops across 7 components on Excalidraw. Plan it as a spec with sol gating. It touches `src/ast.rs` and call resolution, so Tier-A applies.

**STOP conditions:** The owner has not chosen the next seam. Any red CI on #322, #323, or #324 goes back to the owner, not to a re-baseline.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| #322 gate natives (`feat/gate-natives`, `e4491ea6`) | pending owner merge | `[MEASURED]` Full gate on main plus natives: 844/842/0/2 (task `b6grtms3l` output) |
| #323 observer (`feat/native-positional-gap`, `b5af796c`, base #322) | pending owner merge | `[MEASURED]` Stacked full gate: 854/851/0/3 (task `bwjdaewlf` output) |
| #324 entry/call proof readout | done; PR open | `[MEASURED]` `docs/eval/entry-call-proof/{readout.md,receipt.json,extracts/}`. Re-running `extract.py` on both runs matches the committed extracts. |
| Spec review | done (2/2 rounds) | `[MEASURED]` r1 FIX 1W/1S → r2 FIX 0W/1S. Both folded. |
| Reconciliation | done | `[MEASURED]` sol xhigh RECONCILED (`scratchpad/entrycall-r2-sol.last`; key facts in readout §Custody) |
| Option B (byte-exact observer) | parked, not dispatched | Owner D1 = A. The plan of record is kept in the plan folder. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| memory `project_native_positional_gap.md` | "PARKED AGAIN; resume via PARKED.md" | `[MEASURED]` Already corrected: re-plan (b) shipped as #323, and the entry/call proof #324 recorded a defer. |
| Plan README / PLANNING-PROBES P5 | All components attributed to wrapped-export drops; zoom arguments listed as identifiers only | `[MEASURED]` Already corrected in `60cbcba8` (spec r2 SMELL fold). |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Merge #322 → #323 → #324 | pending | Owner merges; controller verifies the retarget | owner | PR 322, 323, 324 |
| 2 | Next seam (wrapped-export resolution) | not started | Owner chooses; then spec it with sol gating | owner | `src/ast.rs:2874-2890` |
| 3 | Clone cleanup | pending | Remove `~/code/prism-{gate-inputs,gate-inputs-review,gate-inputs-review-terra,native-gap-review-kimi,native-gap-review-sol,plan-entry-call-review}` after the merges | #1 | — |

## 5. Invariants and traps — do not do these

- Never publish an extract count without re-running `extract.py`, because the first version counted the literal `true` as an identifier and inflated Y to 3.
- Never use an unquoted heredoc for Python that contains backticks under zsh, because command substitution silently strips the spans. Quote it with `<<'EOF'`.
- Never `git commit -a` in a clone where an implementer is mid-edit, because it sweeps in their WIP.
- Keep evidence out of `/private/tmp`, because the macOS daily cleaner purges files not accessed for 3 days. Use `~/prism-evidence/`.
- Presence claims from `dfg-stats --edges` lack byte and function identity. Only absence over the superset key is sound.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Release binary | `263f71adf43286f70861d9ac649e5aae2fcf5fb7b2f285540b9ba86533acf3b1` |
| targets.json | `b584bb4abc8af77deae06044e633bbd58bd0f066a0c8e7660818cde3d7371ffd` |
| Run MANIFEST (both runs) | `20164e855efded87950a9919a81311470af8fb0c43729b432a256a01d4a0eb9f` |
| extract.py | `9efdf7ba86352a531f90f78469dba7ca761767cc2dbfedf6cb72905461a6c533` |
| Evidence root | `/Users/wesleyjinks/prism-evidence/entry-call-proof/appendix-a/` |
| Input tree | `/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source` |
| Production source commit | `7e11593414ffa7f35711555469bcc74428641f66` (prod diff vs `7ecccd99` is empty) |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "Y ≤ 1, S ≤ 1, so defer" · pass: INDEPENDENT (sol static reconciliation) · evidence tier: STATIC-ONLY · record: `docs/eval/entry-call-proof/readout.md` §Custody

**Questions the owner owes an answer to:**
1. Is wrapped-export resolution the next seam to plan, or is another lane preferred?
