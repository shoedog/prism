# Handoff — Prism roadmap item 2 reaching-definitions closeout

**Written:** 2026-09-05 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-item2-sol` · branch `item2-dataflow-confidence`

**Custody tuple:** `[MEASURED]` final branch head is the single closeout-docs commit `SELF`, directly above rebased candidate `30d8091dbdb3836817890f5af3c93b0020593603`; together they are 36 commits above base `eb884824efc1686da2e789248783afe089c2cd14` (35 rebased code/doc commits plus `SELF`). Last source-changing commit is `bf010d28965e63476e369b1b00b0f704f3b8d0a5`. Rebased Task 0b measurement/control commit is `efadb600add41796a8ca311030f23cfb66657ce8`. Because a commit cannot contain its own SHA, identify `SELF` with `git log -1 --oneline`. Final proof is `/Users/wesleyjinks/code/tools/logs/item2-task7e-step14.log`.

## 0. Gating facts

- `[MEASURED]` `github/main` was fetched and pinned to `eb884824`. Safety tag `pre-rebase2-aeadb5a9` protects the old head.
- `[MEASURED]` Manual conflicts occurred only in `src/cpg_cache.rs`, on the `1355a7f7` and `01ecf2fa` replays; the expected `2b0b10e5` cache replay auto-merged. Main's v73 callable-interface entry and downgrade test remain; Item 2 is the single v74 transition. Navigation sidecar 42 is byte-identical to main.
- `[MEASURED]` No sub-agents were spawned. No push or PR mutation was attempted.
- `[MEASURED]` Disk remained above 8 GiB during the applicable preflight, so no incremental directory was pruned; it never crossed the 3 GiB stop floor.
- `[MEASURED]` Generated Tier-A date report/snapshot files were removed before final custody; no baseline or oracle was edited.

## 1. Resume order

1. Run `git rev-parse HEAD`, `git rev-list --count github/main..HEAD`, and `git status --short --untracked-files=all`. Expect `SELF`, 36 commits, and a clean tracked/untracked tree.
2. Read `.superpowers/sdd/2026-09-04-prism-item2-dataflow-confidence/task-7-report.md` and `/Users/wesleyjinks/code/tools/logs/item2-task7e-step{1..14}.log`, including `step6b`.
3. Before new binding semantics, read binding spec v6.5.6 and obtain a ruling for B-open. Keep B8 provisional.
4. Push/PR is authorized, but remains controller-owned. Force-push PR #254 for this rebase; do not merge.

**STOP conditions:** wrong base or commit count; dirty tree; changed sidecar; byte-control difference; pre-existing call-stats difference; new quick invalid reason; source change during closeout; disk below 3 GiB.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Rebase | done | `[MEASURED]` 35 commits replayed onto `eb884824`; manual conflicts only in the two expected cache replays, with `2b0b10e5` auto-merged; two stale closeout commits folded into `SELF`. |
| AST union | done | `[MEASURED]` Main's `js_ts_local_callable_type` and Item 2's `statement_wrapper_kinds` both survive; focused selectors and full suite pass. |
| Steps 1–3 | done | `[MEASURED]` Provisional clean closeout `35d409eb`; fmt 0; same-worktree clippy `177/177`, normalized delta 0. |
| Steps 4–7 | done | `[MEASURED]` 23 selectors `443/0/0`; **4,197 passed across 31 result lines including the 2 doctests**, 0 failed/1 ignored; byte controls `1,645/1,645` and `280/280`; cache selector `2/0/0`. |
| Step 6b | done | `[MEASURED]` Base→Task 0b Phase 0 is `1,645/1,645` byte-identical; retained Task 0b packet and unchanged corpora pins recorded. |
| Step 8 | comparative pass | `[MEASURED]` Matrix `159/0/0`; both quick runs invalid only for corpus SHA drift and C-name `4/6`; normalized non-matrix diff 0. |
| Steps 9–10 | done | `[MEASURED]` Call-stats `11/11`; Exact/Killed nonzero: prism `88,523/2,342`, caddy `42,343/14,691`, mypy `111,747/9,956`. |
| Steps 11–12 | done | `[MEASURED]` Cache 73→74 once, sidecar 42, Cargo diff 0; four producers, nine delivery cases, six labeled walks, four valid JSONL records. |
| Steps 13–14 | done | `[MEASURED]` Main roadmap change since `04bb5583` is prose-only; Item 2 rows remain 21–22; row 21 says v6.5.6 and cache v74; this handoff uses `SELF`. |

## 3. Corrections and convergence

- A Step 6 wrapper conflicted with the scripts' process substitutions and emitted `Bad file descriptor`; that run was inadmissible. Plain-pipeline reruns produced the accepted 1,645 and 280 results.
- The first Step 7 selector named the wrong module and selected zero tests; it was inadmissible. The corrected `dfg_label_store_test::cache_` selector passed two tests.
- The quick summary initially queried absent `.status` fields; the artifacts use `.outcome`. Corrected summaries show candidate 159 ok and control 104 ok/55 expected-gap. Recursive array sorting plus removal of matrix, binary identity, and timing yields non-matrix diff 0.
- The brief mentions an Item 2 force-previous-version test using 73. Pre-rebase history contains no such Item 2 test; its version commit changes only history/current-version and the pin assertion. Main's separate downgrade test still forces 72, while Item 2's pin now asserts 74. No new source test was added during the rebase.

## 4. Open work

| Work | State | Required next action |
|---|---|---|
| Push and PR | authorized 2026-09-05 | The controller force-pushes `item2-dataflow-confidence` to update PR #254 against `main`; do not merge. |
| New binding semantics | parked | Obtain a binding-spec amendment under B-open. |
| B8 capture rule | provisional | Keep `CfgIncomplete` behavior pending an owner ruling. |
| Tier-A absolute validity | outside this slice | Re-anchor the corpus SHA and restore C-name 6/6 before claiming an absolute quick pass. |

## 5. Invariants and traps

- Ownership must be proved before Exact or Killed; unclassified route-dominating introductions use the unconditional flat-path kill equation.
- Do not add binding semantics without a table ruling, or treat B8 as settled.
- Do not remove any call-stats field except additive `.dfg_labels` in compatibility comparison.
- Do not rebaseline generated 2026-09-05 Tier-A artifacts.
- Keep cache v73 and earlier intact; Item 2 is exactly v74. Keep sidecar 42 unchanged.

## 6. Identifiers

| Item | Value |
|---|---|
| Merge base | `eb884824efc1686da2e789248783afe089c2cd14` |
| Safety tag | `pre-rebase2-aeadb5a9` |
| Rebased candidate (35th commit) | `30d8091dbdb3836817890f5af3c93b0020593603` |
| Last source-changing commit | `bf010d28965e63476e369b1b00b0f704f3b8d0a5` |
| Closeout-docs commit | `SELF` |
| Task 0b source behavior commit | `d0daf21b3ad465fcd77a5ebd07b7ea42156d8b7d` |
| Task 0b measurement/control commit | `efadb600add41796a8ca311030f23cfb66657ce8` |
| Base control | `/Users/wesleyjinks/code/tools/bin/prism-base-eb88482` · source `eb884824efc1686da2e789248783afe089c2cd14` · SHA-256 `17592c8a61ff9394375c3c4de7dce42d90e985d3ac5725b3d432dfd321e39f65` |
| Task 0b control | `/Users/wesleyjinks/code/tools/bin/prism-item2-0b-rebased2` · source `efadb600add41796a8ca311030f23cfb66657ce8` · SHA-256 `9c25d4525a3a6ab4fb18ff983137171bce069e7a36c84f7e81775bea38317c1d` |
| Gate candidate binary | provisional docs-only head `35d409eb68fb7a8f4b61ffa3647fa507bd97c765` · SHA-256 `f177d87ebc1b22c11daec1b8052d54a7cec5bf1c4794583cd43ef06b179cef8b` |
| Byte-control populations | Phase 0 `1,645/1,645`; Item 2 `280/280`; zero differences |
| Cache versions | CPG 73→74; navigation sidecar 42 unchanged |
| Gate logs | `/Users/wesleyjinks/code/tools/logs/item2-task7e-step{1..14}.log`, including `step6b` |
| Task 7 report | `.superpowers/sdd/2026-09-04-prism-item2-dataflow-confidence/task-7-report.md` |

## 7. Verdict and owner questions

### Rebased 35-commit branch list

1. `fe11ad8f22a90383f60a83c1e983f38870d20f6c` — chore(item2): add dfg_census example for the RD cap measurement (spec §12 Q1)
2. `606300636ac26203ec55dd5bd1477e8c65780ee2` — docs: item 2 design (spec v6.2) and implementation plan (v4) — DataFlow confidence via reaching definitions
3. `6084a0b4ebe66cdfc65d801b571ee4da42cee867` — feat(item2): tag CFG sequential edges with lexical arm ids
4. `797354df1810250c18d55f2b936f6dc629ec2437` — fix(item2): flag only sequential joins between incomparable lexical arms
5. `d0daf21b3ad465fcd77a5ebd07b7ea42156d8b7d` — Complete per-language CFG statement universe
6. `448194c0aac4c09fe5366a0cb5a0fd63b6c8ace2` — docs: item 2 design v6.4 (Task 0b metric attribution, exclusion control, byte-control blind spot); plan v4 sync
7. `efadb600add41796a8ca311030f23cfb66657ce8` — Measure distinct-statement DFG admissibility
8. `1233bd2c69ab4c5065d9f4758c5c2656b803bf12` — feat(item2): carry confidence on DataFlow edges
9. `58668b25f5ca5b116c19266b5432ad7639d450f9` — test(item2): add reaching-definitions RED suite
10. `b62c151cafe42f0a18694024c69b693e681045ce` — feat(item2): add the reaching-definitions core
11. `253f5669d25039b58cb43dbd6067264dd6ad2a58` — fix(item2): harden reaching confidence
12. `c2dc4646cf22794a3cd7964594ddfc5288e26240` — test(item2): stage reaching-definition matrix fixtures
13. `0c6181f71667402ea842993d4ef7731a63e8a323` — feat(item2): label existing DFG edges with reaching defs
14. `24f2566daad2515624976106c14ab5a4045be460` — fix(item2): keep dfg matrix command failures visible
15. `86691b833f494437a9974fead063b23e060eb855` — fix(item2): harden DFG label lifecycle
16. `7516de759db65a1276a4d6892d677d788ae61a26` — fix(item2): preserve RD fallback identities
17. `f5512184430d3315bffb46f6fa9b6983f4c0edd4` — feat(item2): classify findings from selected evidence
18. `b490039c9d243581791cd262086224ababb1c06f` — fix(item2): preserve selected evidence confidence
19. `8addb5dd248e15a9981852ad3898287a1b6527dd` — feat(item2): filter emitted findings by confidence
20. `ef6dd374d3543e1ca92d16e33662f7d8b1286c31` — fix confidence admission and byte controls
21. `d827bb905505b808a33a92b272978b2871cd8e17` — feat: complete Task 6a dataflow confidence gates
22. `97945dc783adc79707402dedcfba76da7df6b962` — wip(item2): Task 6a fix round 1 — binding fallback + spec v6.5.2 mirror
23. `9b46a0fbe005ac4a92e2b269bad6f52875bb3af2` — fix(item2): honor language-specific binding scopes
24. `6774718f5cf15c20a7da9e8f77802ea83f3bcd7f` — fix(item2): finish Task 6b truth and binding pass
25. `51bab015b42602b241d7411364c0ed601c8b2a07` — fix(reaching): respect Python comprehension iterable scope
26. `ee72d352bdf805a81a79428af3891bf3a4c42a0e` — test(item2): fix cloned slice clippy warnings
27. `098c3d99b644efa6a2e7a13ede8f96ae17631d15` — test(item2): record reaching-definitions closeout
28. `e6e78bb62dd6adf8b5e338703eb6a3ac633c8999` — docs(item2): handoff custody tuple — closeout docs are committed on top of 357221b; tree clean
29. `3b14dfe8d04111f5db523b46550cdb9a325c0e14` — docs(item2): handoff custody — tree clean at the closeout-docs commit; docs committed
30. `100e43627e81124fbe29867e7d944eb73c7bdec9` — docs(item2): handoff custody line — tree clean at the closeout-docs commit
31. `83daa7c7d0947ebf548e5e7370dd680d4f658597` — fix(item2): fail closed on uncertain bindings
32. `21e7b8f11df2eeb81e2afa3b6e8393cdf45490d1` — fix(item2): keep reaching solver clippy-clean
33. `578b4f2df37819ac95e0e2afc44b69ab5d5a5c48` — docs(item2): close whole-branch fix wave 1
34. `bf010d28965e63476e369b1b00b0f704f3b8d0a5` — fix(item2): derive uncertain bindings from grammars
35. `30d8091dbdb3836817890f5af3c93b0020593603` — docs(item2): close whole-branch fix wave 2

**Verdict:** SURVIVED · Full rebase-2 gates pass at the provisional closeout tree over `eb884824`, subject to the shared Tier-A absolute-validity exclusions and the disclosed absent force-previous-test lineage. Evidence is test-backed, byte-controlled, and same-base controlled.

**Owner questions:** settle B8 before broader capture semantics; assign the Tier-A re-anchor outside this slice. (Push/PR: authorized 2026-09-05; executed by the controller.)
