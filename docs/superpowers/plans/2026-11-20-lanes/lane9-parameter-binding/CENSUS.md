# Lane 9 census (stage M), base build 06157400a3be (product code identical to 9d1a7d06f)

Corpora: X = Excalidraw 0642e72c, T = TypeScript sources 7964e22f, SB = 594 extracted benchmark package roots (600 listed, 6 without an extracted source root), G = gin (control, no JS). Grader: lane 9's adjudicate9 (self-test 114 rows, 0 mismatches), checker TypeScript 5.9.3. Raw outputs and scripts: `~/prism-evidence/param-binding/` (`census/M*/<corpus>/aggregate.json`, `m3/`, `m6/`).
Counts are sites that can receive an added edge (conversion candidates), not graded credit; graded credit is reported where a grader mode was run.

## Per measure

| Id | X | T | SB | Total | Notes |
|---|---:|---:|---:|---:|---|
| M1 sites with a spread followed by another argument | 3 | 3 | 4 | 10 | all 10 resolve nowhere today (dropped); no post-spread row exists on any corpus |
| M1 rows emitted today | 0 | 0 | 26 | 26 | all 26 are chain rows (a use outside the resolved call's span, 7 sites); graded: 18 unproved, 8 chain, 0 correct, 0 proved-wrong |
| M2 gate sites (Exact callee, identifier rest formal with a Def, variable argument reaching the rest index) | 6 | 314 | 162 | 482 | cross-file free_single sites: 0 / 1 / 0 |
| M3 rows added by lifting D12 (probe) | 0 | 68 (19 exact, 49 name-only) | 127 (123 exact, 4 name-only) | 195 | see below |
| M4 gate sites (table method, function argument, fed formal read, receiver node exists) | 421 | 321 | 3,168 | 3,910 | receiver-absent: 903 / 748 / 8,463 |
| M4 typed receivers resolving to the library declaration | 339 of 339 | 316 of 321 | 681 of 696 | 1,336 of 1,356 (98.5%) | repository declarations: 0 / 5 / 15; untyped (no declaration): 82 / 0 / 2,472 not counted |
| M4 role sample (200 per corpus, builtin mode) | 150 C / 0 W / 50 U | 3 C / 0 W / 197 U | 47 C / 0 W / 153 U | | UNDECIDED = receiver element transfer or signature not witnessed by the checker |
| M5 gate sites (parameter call site with payload Use, 1 to 8 targets from admitted Exact callers, anonymous targets included) | 45 | 354 | 854 | 1,253 | ArgFanout drops with payload: 6 / 37 / 62 |

M3 (D12 repeat) detail: rows graded in `binding` mode. T: 68 rows, 0 correct, 6 WRONG (all name-only), 62 UNDECIDED (19 exact rows, 14 of them at cross-file free_single sites that binding admission removes; the checker cannot witness the runtime callee). SB: 127 rows, 116 CORRECT, 11 UNDECIDED, 0 WRONG; 114 sites, 104 of them with every added row CORRECT. X: no row changes. Wrong Exact rows: 0 on all corpora. The three PROBE-LOG P11 rows (`debug.ts` `Array(height)`, `incrementalUtils.ts` `readFile(fileName)`, `deprecations.ts` `bind(args)`) are not among the added rows on this build.

M6 (benchmark on the base binary, all classes): traced 252, function-only 57, partial 40, prism error 22, not reached 2, ground truth unavailable 210, acquisition excluded 17 (of 600). First-break attribution of the non-traced: B-plain-argument 20, E-admission 19, B-arguments (`arguments`, no parameter Def) 3, C-member 3, D-cjs 6, H-callback/promise/event 4 (of which `callback_argument_parameter_registration`: 2), unresolved 63. No case is labelled first-broken at a rest Def. The handler-registration cases that the plan expected (97) now trace; 2 remain, to be handed to lane 12.

## Go or no-go (DESIGN section 6)

| Slice | Threshold | Measured | Decision |
|---|---|---|---|
| A | always | removes 26 chain rows (0 correct) on SB; no spread row at a resolved site on X, T | GO |
| B | M2 >= 300 sites, or M6 >= 5 | M2 = 482 | GO |
| C | no checker-WRONG Exact among the edges binding admission keeps | 0 WRONG exact on X, T, SB; 104 SB sites all CORRECT; T's exact rows UNDECIDED only | GO |
| D | M4 >= 1,000 callbacks and >= 90% typed receivers resolve to the library method | 3,910 sites; 98.5% | GO |
| E | M5 >= 300 sites | 1,253 | GO |

## Parent-gate forecast (DESIGN 6)

Predicted conversion sites by slice (union, each site once; gate counts only checker-CORRECT added rows, so these are upper bounds until graded): B 482, C 104 (SB, all rows CORRECT), D up to 3,910 (role sample: CORRECT where the checker witnesses, otherwise UNDECIDED), E up to 1,253 (graded by the argfollow mode after E1). B + C alone exceed 500; forecast passes. Predicted new benchmark traces: none attributable (M6 shows no first break at a lane shape other than the 2 handler cases and 3 `arguments` cases outside this lane). Re-forecast after A, B and C.

## Caveats
- X and T M3 UNDECIDED exact rows earn no credit (DESIGN owner decision 6).
- M1's spread sites are rare on these corpora (10 sites); slice A is a correctness fix, not a yield slice.
- The checker cannot witness runtime callees for much of T (compiler sources use own helpers), so T grading is mostly UNDECIDED.
