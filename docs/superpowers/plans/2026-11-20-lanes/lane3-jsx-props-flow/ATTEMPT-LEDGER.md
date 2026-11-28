# Lane 3 (JSX props flow): attempt ledger

Every slice, fold and review round of the lane, with its outcome. Sources: the lane `LEDGER.md` (timestamped
log, 188 lines) and the lane `HANDOFF.md`. "Cap" is the round cap declared before the loop started. A round
"past the cap" is a round that ran after the cap was reached, with the reason. Reviewers: A = Opus-5.5 high,
B = gpt-6.2-sol xhigh (astra on S1 code round 1 to 2), unless stated. Dates are 2026-11-24 to 2026-11-27.
No private information appears here.

## Totals

| Item | Count |
|---|---|
| Pull requests merged for the lane | 9 (#904 design, #907 matrix, #925 and #934 design amendments, #929 S1, #949 S1 test-seam fix, #952 S2a, #959 S2b, #964 S3) |
| Slices with product code | 4 (S1, S2a, S2b, S3) plus one test-seam fix (#949) |
| Design-of-record review rounds | 4 (cap 2): r1 to r3 both reviewers, r4 one reviewer on one finding |
| Enumeration rounds (stage E) | 15 (terra 5, sol 5, astra 5; cap 5 per tier), 338 matrix rows at the end |
| Review rounds on the first S1 attempt (never merged) | 2 (cap 3), then escalated to a design amendment |
| Design amendment PRs | 3 (#925: 3 review rounds, cap 2; #934: 5 review rounds, cap 2) plus the 5.3.4 amendment inside #964 |
| Review rounds on S1 (#929) | 3 declared rounds plus 2 single-finding closure checks = 5 |
| Review rounds on S2b (#959) | 4 (cap 3, one disclosed extension) |
| Review rounds on S3 (#964) | 6 (caps: 3, then 2, then a closure), all of them listed below |
| Rounds past a declared cap | design of record +2, #925 +1, #934 +3, #929 +2, #959 +1, #964 +1 (the closure round after the cap-2 rounds); 10 in all, each with a recorded reason (converging findings, or an owner or controller decision) |
| Times the gpt-6.2-sol seat was stopped by the provider's content filter | 3 (jobs 093 and 094 on #964 round 5, job 095 on the closure round). On the closure round the second seat became Sonnet-5.5 high (owner-approved fallback) |
| Owner or controller decisions that changed the plan | 6: S1 rule D (astra escalation), S1 aliasing amendment, S2b strict accept-list, S3 rule R (11 edges) priced then replaced by P2 (notice 33.4; 50 of 1,992 candidates withheld in all), cap extensions (notices 27 and 29), the sol-seat fallback |

## Stages M, T, E, D

| Stage | What | Outcome |
|---|---|---|
| M (census) | Census of four public corpora on the pre-lane product, M-3 tools | GO: 1,018 predicted connected pairs against 1,000 (Excalidraw 308, cal.com 710); rates 36.07% and 38.34% against 30% |
| T-1 | First matrix, 110 rows | 89 failing, 21 passing before any code; two rows had plan labels contradicted by probes |
| E (enumeration) | terra 5 rounds (10, 8, 5, 5, 4 new rows), sol 5 rounds (27, 19, 17, 13, 9), astra 5 rounds (26, 21, 14, 12, then the closing round) | 311 rows; each tier shrinking; the controller closed stage E after astra round 5 |
| D (design) | Draft 1 by astra, then fold r1, fold r2, fix r3 (4 drafts). Reviews: r1 A reject, B reject; r2 A approve, B reject (React member mutation re-graded out of model, the rest folded); r3 A approve, B reject (1 finding, a closed set); r4 B approve | Merged as PR #904 (7f9264ca). Cap 2 rounds, ran 4 (converging: 7, 5, 1, 0 material findings), disclosed |
| Matrix PR | 338 ids (311 + 27 added in the design folds) | PR #907 merged ada0938a; one Sonnet-5.5 high reviewer approved |

## S1: pattern formals (Defs for destructured props), PR #929

| Step | Outcome |
|---|---|
| First attempt, branch feat/jsx-props-s1 (b481a6dd, e75f75dc, 0085a4e4) | r1: A approve, astra reject (7 material findings); fold r1; r2: astra reject (6 material, an open class: reads and writes inside the formal-parameter list). Two of the declared three rounds; classified open class, escalated under ORCHESTRATION section 8. This attempt was superseded, never merged |
| Escalation | astra (max) options, a pooling job, then docs PR #925: r1 A approve, B reject (1 wrong); fold 1; r2 B reject (1 wrong, partly closed); fold 2; r3 B approve. 3 rounds (cap 2). Merged 418ffa6b |
| Implementation (sol xhigh, scoped brief) | PR #929 opened 2026-11-25 16:04 |
| #929 round 1 | A reject, B reject (9 material): classes entry projection, legacy alias twin relabel, JSX representative |
| Pricing | M1 rule demotes 3.4 / 5.1 / 7.3 percent of all added edges (Excalidraw, TypeScript src, cal.com), below the one-third stop; 0 of 30 hand-read edges were projections |
| Aliasing escalation and amendment PR #934 | astra recommendation, sol drafted; r1 A approve, B reject (8 findings, 3 wrong); r2 B reject (4); r3 B reject (2, fixture expectations only); r4 B reject (1, my own replace hit three other dispositions); r5 B approve. 5 rounds (cap 2): findings 8, 4, 2, 1, 0, converging, each a closed set. 87 of 87 retained projections hand-read CORRECT. Merged f415f9b4 |
| #929 round 2 | A reject, B reject: one class, legacy neutrality |
| #929 round 3 (declared last) | A approve, B reject (1 closed wrong: compound computed-key writes); closed fold; r6 B reject (1 wrong: nested skipped key); closed fold; r7 B approve. 2 closure rounds past the cap, both disclosed (findings 5, 1, 1, 0) |
| CI | codecov/patch 91.6% (257 lines): trace tests added; SiteFacts size budget and a probe count fixed |
| Merge | PR #929 abc5b2f9 (Opus approved 7e51c0db, sol approved the final head) |
| Closure | Opus approve, a separate sol reject on test-seam defects only (production unchanged with the switch unset). Fix PR #949: sol r1 reject (2), r2 approve. Merged 413e4501, sole reviewer sol, disclosed |

## S2a: argument binding for plain-literal calls, PR #952

Opus and sol approved round 1 of cap 3 on 3f2ea40c. The differential on four corpora was identical.
codecov/patch failed (21 lines); four tests added (7 lines unreachable by any input, reported). Merged 6179c7e1.
One round, one coverage fix.

## S2b: object-literal arguments through wrappers and hooks, PR #959

| Round | Outcome |
|---|---|
| r1 | A approve; B reject (2 material: a `.bind` time-of-definition gap, the oracle certifies excluded hook shapes) |
| r2 | A approve; B reject (an open class of definition-order bypass forms). Not converging: notice 29 item 2 applied |
| Pricing and decision | pre-initializer sites 6 / 0 / 24, accept-list admits 5 / 0 / 18, loses 2 / 0 / 20 edges; controller adopted the strict accept-list |
| r3 (cap 3 reached) | A approve; B reject (3 oracle findings, none fired on the corpora). Split at the cap, reported |
| r4 (one disclosed extension) | A approve; B approve on fa09cee1. Merged e37d644d, 13 of 13 checks |

## S3: JSX element props flow, PR #964

| Round | Outcome |
|---|---|
| Entry gate | 959 pairs / 1,086 candidate edges against 1,000 (margin 86) |
| Tests first, implementation, fix 1, fix 2 | tests-first commit bf14aa4a; implementation checkpoint 155 of 179 rows; 24 rows and 362 generated cells adjudicated as fixture-wrong by an independent job |
| Oracle | r1 found 3 / 80 WRONG; classed an oracle fault (4 samples hand-verified true), the oracle was rebuilt independently (r2, r3, r4) and the final runs have 0 WRONG |
| r1 (cap 3) on 23d9c9d4 | A reject, B reject (class-body this/super stale source; destructuring-assignment order; oracle gaps) |
| r2 on 30f1a045 | A reject, B reject (open class of write-model forms). Notice 29 item 2: priced rule R (11 of 1,992 edges) |
| r3 (cap) on e63fdd66 | A approve, B reject (mapped `arguments` writes, oracle overcredit). Split at the cap |
| Closing fold | sol drafted a 29-class inventory; astra rejected it as not complete (nine new classes). Stopped at the design check, no product change |
| Owner decision (notice 33.4) | Rule R replaced by the priced positive certificate P2 (29 of 1,981 edges lost), plus DESIGN amendment, tests first, one choke point |
| r4 (cap 2) on 601bb1c9 | A reject, B reject (await receiver, line-terminator hidden write, type-query call args). Folded as one clause |
| r5 (cap) on eadd4374 | A reject (2 material, one open class: grammar and language disagree on ASI); sol stopped by the filter twice (jobs 093, 094). Stopped, reported |
| Structural fold | a structural rule for clause c (tree-sitter and TypeScript agree), measured first: cross-check 0 genuine differences on all 1,942 kept edges, price 10 of 1,942 before admitting two object kinds, 0 after |
| r6 closure on 25459f77 | A approve; sol stopped by the filter a third time (job 095); second seat Sonnet-5.5 high approve |
| Merge | f720d717 with ad35f43c (a doc cite and a comment word differ from the approved head). CI 13 of 13. Footprint guard: prime LOADED 3,143.5 MiB, guarded UNLOADED 16.1 MiB, control UNLOADED 2,006.4 MiB |

## What this ledger does not contain

The per-site attempt ledger of DESIGN 5.7 (join of every JSX site, the three ambiguity buckets, unjoined
sites, trace boundaries) was not materialized as one table; the product's own counters and the oracle
verdicts carry the same refusals (REPORT section 4 gives the counters; the pieces that are `unmeasured` are
listed there).
