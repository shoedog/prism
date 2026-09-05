# Handoff — PR 243 main conflict resolution

**Written:** 2026-09-04 · **By:** Codex /root · **Provider:** codex
**Workspace:** /private/tmp/prism-pr243-main-drjoY2 · fix/pr243-main · **Measured state:** `[MEASURED]` merge b0cf1a24eff04888e8311f0f3abdc93b74c82fb3 pushed to targets-dependency-hint; parents f751a756 and main854d53f. GitHub confirms PR243 OPEN and mergeable, not merged. This follow-up records publication only.
**Predecessor:** PR #243 targets-dependency-hint; #244 remains separate and mergeable at initial GitHub check.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) Ownership: `[MEASURED]` isolated worktree created for this session; original #244 checkout and untracked files untouched — RESOLVED.
(b) Custody: `[MEASURED]` merge b0cf1a24 pushed; raw evidence archived below; worktree clean before this documentation follow-up — RESOLVED.
(c) In flight: `[MEASURED]` full suites/release/matrix/quick complete; no pending process — RESOLVED.
(d) Authority: owner “can you resolve conflicts with PR 243 and main”; update #243 branch, do not merge either PR into main.

## 1. Resume order

1. Read /private/tmp/prism-pr243-verify-YYpo2A/default.log, all-features.log and release.log; run git status --short in this worktree.
2. Review PR243. GitHub confirms mergeable; no automatic merge. PR244 is unchanged and an object-only merge-tree compatibility check with its head5b98bee returned a clean tree (not a combined behavior test).

**STOP conditions:** three review rounds maximum; unrelated test failures require same-environment base control before attribution; no rebaseline or automatic PR merge.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Conflict diagnosis | done | `[MEASURED]` only roadmap row19/20 conflicted; source merge automatic |
| Resolution | done | `[MEASURED]` keep main row19, retain PR243 row20; both integration test registrations present |
| Full suites | done | `[MEASURED]` default3803/0/1; all-targets/all-features3991/0/1 plus doc2/0/0; logs under /private/tmp/prism-pr243-verify-YYpo2A |
| Static and matrix | done | `[MEASURED]` fmt clean, Clippy complete with warnings; matrix104/104; dependency-hint source/tests identical to PR243; all19 main and18 PR243 integration module registrations retained |
| Quick | done | `[MEASURED]` exit2 solely SHA drift f751a7569ea1 vs pinned20c8490591a3; oracle quiescent, oracle/SUT errors0, stale adjudications4; run label2026-09-04-pr243-main |
| Publication | done | `[MEASURED]` b0cf1a24 pushed to targets-dependency-hint; PR243 OPEN and mergeable |

## 3. Corrections to standing documents and memory

Initial assumption that #244 needed conflict work was corrected by GitHub: #243 conflicts, #244 mergeable. No memory edits. Roadmap keeps current main Phase0 closure instead of obsolete PR243 row19.

## 4. Open work

Owner review/merge only. Existing PR243 census/review claims are `[INHERITED]` from its PR body, not independently repeated by this conflict-only task.

Quick is not a green accuracy gate. Pins: target-c-method flip_candidate
(Exact TP5/FP0/FN0, 28 extra default candidates); module-deps-feature-gated missing
literal pin (oracle-only empty, MCP tools:230 Prism-only); load-repo-feature-gated
missing literal pin (oracle-only resolution_test.rs:5299/5368, MCP freshness:401
and session:359/374/400 Prism-only); ambiguous-symbol ok. No same-environment base
quick was rerun for attribution, and no baseline or full multicorpus was changed.
The manually resolved change is documentation-only; no new behavior requires a
new RED test. Both parents' existing behavior tests ran in the combined tree.

Whole-merge diff-check reports four existing two-space Markdown breaks in main's
python-imported-typed-receiver handoff lines9–12. Same-environment git show/diff
confirms they are byte-identical to main; scoped diff-check against main passes.

## 5. Invariants and traps — do not do these

- Do not replace roadmap wholesale with either side; that loses current row19 or added row20.
- Do not push fix/pr243-main to main or change #244.
- git merge-tree --write-tree writes Git objects; escalate up front. Initial sandbox-refused probe was inadmissible, not conflict evidence.
- No new feature behavior is introduced by the manual resolution; preserve existing tests from both parents.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| PR | https://github.com/shoedog/prism/pull/243 |
| Remote branch | targets-dependency-hint |
| Main | 854d53f49606461421ce8de84a2618aa79adac58 |
| Evidence | /private/tmp/prism-pr243-verify-YYpo2A |
| Evidence archive | /private/tmp/prism-pr243-verify-YYpo2A-evidence.tgz |
| Archive SHA256 | 39ed1ba8e22b73792f213abdd7e8addc1692f7f92fbfdfad850c42c9eb7861c2 |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "merge retains both parents' intended changes" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: row-preservation assertions, source diff, full suites and matrix; round2 of3, no implementation changes or review extension needed. Quick excluded as baseline-invalid above.

**Questions the owner owes an answer to:** None.
