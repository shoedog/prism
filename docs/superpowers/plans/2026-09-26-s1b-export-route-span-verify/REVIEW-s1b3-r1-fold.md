# S1b-3 spec round 1 of 2 (Opus FIX 3 WRONG / 3 SMELL, sol FIX 2 WRONG / 1 SMELL): fold record

Reviews: `~/prism-evidence/s1b/reviews/s1b3-spec-r1-{opus,sol}.md`. Both reproduced the controls, the row-diffs and 3a's
0 rows. Owner decision 2026-09-29 (SPEC §0): narrow the R5 drop to bindings that provably hold no function, apply it
to R4 alike, and re-measure every corpus. Every row below is measured on the **clean-built v11** (scratch `0982f2fd`,
`BUILD-v11.txt`); new controls were pre-registered (addendum 9 `01ac45d3`, 9b `6a295f3b`) before they ran.

| # | Finding | Disposition | Where | Controls and mutants | Measured |
|---|---|---|---|---|---|
| Opus W1 | The R5 reach drops right edges reached through a value alias; OQ12's "0 right lost" was false | **Fixed per owner 2026-09-29:** the closed NoFn class (§3.8 (11)); any other declarator value is `JsBinding::Alias`, which keeps base at R4 and R5, counted `local_binding_may_call: {alias}`; D4 keeps 2b's answer | SPEC §0, §3.8 (4)(8)(11), §5, §8 | C-45: C174–C177, C182–C184 (both grammars); mutants C-M38–C-M40 | v10's alias removals return to base: X 38, F 424, T 212 (Q63). v11 removes 0 declarator bindings; 0 right lost outside E6 (Q64–Q65) |
| Opus W2 | The B1 relaxation sealed a parameter-list error for a body site | **Fixed:** a `formal_parameters` error keeps the r3 rule (the site must be outside the whole sealer); only a body / class-body error uses the delimited child | SPEC §3.8 (6) | C-46: C178; mutant C-M41 | C178 drops (head and v10 Exact); C172 still verifies; 0 corpus rows (3a 0) |
| Opus W3 | Poison fired on `import.meta` / `import(` ERRORs | **Fixed:** a recovered static import needs first token `import` and next token neither `.` nor `(` | SPEC §3.8 (7) | C-42: C179; mutant C-M36 | C179 Exact kept ×2; 0 corpus rows |
| sol W1 | P1 dropped valid non-ASCII identifiers (`é` + U+0301), leaving a false Exact | **Fixed:** the shared `collect_js_ts_binding_pattern_names` trusts parser-confirmed identifier nodes spelled without `\` (B0 still refuses escapes); S1b-1b's `$` kept | SPEC §3.1 P1, §3.8 (12) | C-47: C180 and P1 preservation rows; mutant C-M42 | C180 drops (head Exact). Blast radius of the shared helper: **0 call-site rows on X, F, R, T** (Q66) |
| sol W2 | The poison scanner split a Unicode alias | **Fixed:** every `identifier` in the recovered import plus words over ASCII identifier characters and all non-ASCII non-whitespace characters | SPEC §3.8 (7) | C-42: C181 (`é`, ZWNJ); mutant C-M37 | C181 drops ×2 (head Exact) |
| Opus S1 | 3b's tests under-forecast (≈ 35 × 30 ≈ 1,050 vs cap 800) | **Re-forecast by enumeration:** 36 scenarios × 30 + 120 ≈ 1,200 with the C-5/C-6 guards moved to 3a unit rows (+270 if kept end to end); proposed 3b tests cap 1,320 / 1,190; 3a re-measured 582 src → cap 700 / 630, tests ≈ 670 → 740 / 670 | SPEC §9 | – | Q67 |
| Opus S2 | The auditor shares the planner's alias blind spot | **Fixed:** `removed_alias` class; hand audit of every removed row whose declarator value is an identifier, member or call — **0 such rows** in v11 | `probes/audit_s1b3.py`, SPEC §8 | – | Q63–Q65 |
| Opus S3 | C166–C173 not pre-registered; `js_binding_tests.rs` is 584 lines | **Fixed:** addendum 8 already disclosed; addendum 9/9b pre-registered and committed before running; 3a plans new test files | probes, IMPLEMENTOR | – | the 9b first run was inadmissible (anonymous-arrow callers, no rows) and is recorded |
| sol S1 | v10e binary bound to a dirty tree | **Fixed:** clean build of committed `0982f2fd`, receipt with HEAD, status, sha256, `--version`; every receipt regenerated | `BUILD-v11.txt`, PLANNING-PROBES r4 fold custody | – | – |

## Consequences disclosed to the owner

- **Carry-forward 4 is only partly met.** Its block-nested `const { f } = o` rows (S1b-1b `b5` in-block, F's 2) have an
  identifier value, so under the 2026-09-29 rule they keep base (the wrong edges stay); `b5` passes unchanged.
- **Some r3 R4 removals return to base** where the binding is an alias: X 5, F 28, T 24 rows (for example C114
  `using h = res()`), counted in `local_binding_may_call.alias`.
- **3a's 0-row property holds** after the fold, P1 change included.

## Prior-round closure

- r4 (v10) carry-forwards 1–3, 5–8: closed (both reviewers). Carry-forward 9: closed by W3 and sol W2's folds.
  Carry-forward 4: narrowed by the owner as above.
