# E5 Step 1 — stopped union candidate

Status: **STOPPED / INCOMPLETE / NOT READY FOR REVIEW OR ADOPTION**. Entry/patch base `d01fd65fd0c821bb54887d9c48b4519d96f49776`; main product control `bab0dfbd`. The owner brief is `/Users/wesleyjinks/prism-evidence/js-e5/plan-brief-step1.md`.

## Mandatory STOP

The first complete union suite fails `mcp::transport::tests::lazy_runtime_delegates_each_refresh_policy_after_readiness` at `src/mcp/transport_tests.rs:1060`: a Python-only background provider is not Ready after its one-second initial wait. Its purpose is neither alias policy nor a repairable call-binding fixture dependency, meeting the brief's fourth STOP. No repair, retry, timeout adjustment or design-around was performed. The pre-union control passed this test; resource load differed because the head suite overlapped release compilation. Scheduling delay and builder failure can both produce this output; the cause is unresolved and is not attributed to the union.

The suite and release build were already running at STOP and were allowed to finish for complete enumeration and custody. Before the subsequent user verification hook, no further implementation, public head producer, or gate was dispatched; its limited synthetic-helper extension is recorded below.

## Preserved work

The working tree contains the union predicate at both existing FreeSingle arms. Arm (a) is unchanged. Arm (b) retains caller-file root import/export syntax and syntactic import/require names; full/skeleton/subset construction, serialization, file removal and merge preserve those facts. It refuses cross-file unqualified singleton calls without name evidence, including JSX and aliases in module callers. Positive import/require evidence only preserves prior behavior and supplies no new binding authority. CACHE_VERSION and both registry pins stay 115; D12 is unchanged.

Five new regression groups pass in the first union suite: ordinary module calls, JSX, complete-value preservation controls, bidirectional caller-fact replacement/serialization, and argument/return modeling. Pre-change RED used the step-0 runtime, not a freshly executed bab0dfbd main control. New synthetic fixtures were corrected to exercise reachable outer-scope aliases/requires; no existing test was changed. Product-source snapshot and frozen release binaries are in step1/union-source-at-stop.tar.gz and step1/binary-binding.json.

## Public union measurement status

No real paired union capture was run. For X, T and all 579 admitted SecBench roots: target removals by verdict, (a)-only/(b)-only/both splits, JSX/ordinary split, every call-site/graph-row delta, inventories, navigation/module-dependency changes and non-JS/non-call byte parity are **NOT MEASURED**. No zero-cost, zero-added-row, zero-non-JS-delta or acceptance claim is made. Actual UNDECIDED removals and their per-target/sample disclosures are unavailable because there is no union capture; the historical R-bound experiment touched 34 X UNDECIDED targets, which must be disclosed if the union removes them.

All step0 evidence remains read-only. Reused frozen base binaries were built from 40ad8390; equality of its product source to bab0dfbd is inherited from the controller brief. Every supplied binary hash was checked against step0/final-product-binding.json. Reused base call sites, statuses and fact captures are exposed through step1/base symlinks; graph directories were identified for reuse but no paired graph step ran. The refreshed base census retains all 2965 targets and the complete prior verdict/form/binding cross-tab, with zero overrides; step1/base-census-parity.json verifies that equality. This refresh adds pricing metadata, not head behavior evidence.

The four historical exclusions remain unknown cost: clean-css_4.1.10 killed (exit -9, 394.508s); atropa-ide_0.2.2-2 timeout (1500.021s); natural_5.1.0 timeout (1500.931s); three_0.122.0 timeout (1501.928s). No new producer timing or exclusion was invented.

## R-local pricing and recommendation

| Predicate (census only) | Corpus | WRONG yield | CORRECT cost | UNDECIDED | INADMISSIBLE | Scalar-require CORRECT costs |
|---|---|---:|---:|---:|---:|---:|
| R-local | X | 19 | 0 | 11 | 0 | 0 |
| R-local | T | 148 | 0 | 0 | 0 | 0 |
| R-local | SB | 1249 | 10 | 17 | 24 | 10 |
| R-local-no-require | X | 19 | 0 | 11 | 0 | 0 |
| R-local-no-require | T | 148 | 0 | 0 | 0 | 0 |
| R-local-no-require | SB | 449 | 0 | 2 | 24 | 0 |

The existing census axis classifies imports and destructured require separately, but scalar require as `local_declaration`. Therefore naive axis-based R-local includes ten CORRECT literal scalar-require costs, all in SB. R-local-no-require exempts every require-initialized local (including non-literal/wrapped initializer evidence) and implements the stricter no-require boundary: zero CORRECT cost, retaining 19/148/449 WRONG on X/T/SB (616 total), with 11/0/2 UNDECIDED exposures. It sacrifices 800 WRONG and 15 UNDECIDED SB rows relative to naive R-local while preserving all ten CORRECT costs. These are whole-base census prices; real or marginal post-union yield is not established.

**Recommendation: follow-up.** Do not add another predicate to this stopped, unmeasured union slice. Naive R-local violates zero-CORRECT-cost acceptance; the every-require exemption needs its own bounded runtime/source tests and public/private measurement before adoption. Both predicates now appear in controller census summaries so the controller can price F without disclosing private rows.

## Complete first-suite existing failure population

| Existing test | Classification | Purpose / disposition |
|---|---|---|
| `prism mcp::transport::tests::lazy_runtime_delegates_each_refresh_policy_after_readiness` | STOP | Runtime readiness, Python-only fixture, one-second wait; neither alias policy nor call-binding fixture dependency. Cause unresolved; scheduling delay and builder failure can both fail the assertion. |
| `prism::integration js_binding_scope_resolution_fold2_test::td_duplicate_for_heads_negative_control_single_head_binds` | FIXTURE_DEPENDENCY_UNREPAIRED | Scope/duplicate-for-head negative control relies on an ES-module caller reaching a script-global target. A script caller can retain its exact assertion and purpose. |
| `prism::integration js_binding_scope_resolution_r2_test::rep_unbound_name_through_an_unrelated_parameter_keeps_base_at_r5` | FIXTURE_DEPENDENCY_UNREPAIRED | Unrelated-function parameter scope control relies on an ES-module caller; a script caller preserves the Exact positive control and scope purpose. |
| `prism::integration js_binding_scope_resolution_test::c41_with_reason_keeps_base_at_r5` | FIXTURE_DEPENDENCY_UNREPAIRED | Reason-scoped S1b R5 guard control relies on unimported ES modules. Script caller and script target preserve the assertion for the pre-existing guard. |
| `prism::integration js_binding_shorthand_test::b5_nested_declarators_keep_base` | ALIAS_POLICY_EXPECTATION_UNCHANGED | Explicit nested alias/var fallback policy in ES-module callers. Union deliberately refuses the unsupported FreeSingle paths; imported scope controls must retain complete values. No change made after STOP. |
| `prism::example/parameter_slot_characterization tests::d7_predicate_and_singular_parameter_rows_are_complete` | EXCLUDED_INHERITED | Explicit brief exclusion; same assertion failure in pre-union control. Not fixed or counted against union acceptance. |

**Changed existing test expectations: none.** Three fixture-dependent scope/reason controls and the explicit shorthand alias-policy expectation are classified above but were not repaired after STOP. The MCP test is untouched. The brief-excluded example is unchanged and not counted against the head's acceptance failures; raw suite totals still include it. The reserved `slice_elem_variant_reserved` test is the sole skip. Full test-name and assertion receipts remain in step1/failure-population.json and union-full-tests-1.log.

## Gates

| Exact command / required gate | Result | Evidence under step1 |
|---|---|---|
| `cargo nextest run --offline --all-targets --features mcp` | PRE-UNION: 5293 run, 5292 passed, 1 failed, 1 skipped; 248.111s | pre-union-full-tests.log |
| `cargo test --offline --lib resolution::e5_bound_tests -- --nocapture` | Three pre-change synthetic test runs; four intended refusal groups RED after reachable-fixture correction. Initial preservation failures were bad fixtures. Final main-RED verification NOT RUN. | step0-runtime-bound-red.log; step0-runtime-bound-red-2.log; step0-runtime-bound-red-final.log |
| `cargo fmt` | Applied formatting before first union suite; not a cargo fmt --check gate | No separate log |
| `cargo nextest run --offline --all-targets --features mcp --no-fail-fast` | UNION: 5298 run, 5292 passed, 6 failed, 1 skipped; 283.890s. All five new groups pass. Complete failures below. | union-full-tests-1.log |
| `cargo build --release --offline --bin prism --example e5_step1_facts --example e5_step1_graph` | PASSED, 2m19s. Dispatched before STOP; three frozen head binaries with SHA256 manifest. | head-release-build.log; binary-binding.json |
| `cargo fmt --check` | NOT RUN: STOP | None |
| `cargo test --doc --offline --features mcp` | NOT RUN: STOP | None |
| `python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --since d01fd65f --scope fn --authoritative --jobs 4 --out /Users/wesleyjinks/prism-evidence/js-e5/step1/mutants` | NOT RUN: STOP; new predicate mutants not added. Both existing cache pins stay 115. | None |
| `node --test docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-e5.test.cjs docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/controller-parity.test.cjs` | NOT RUN: STOP. Actual-shell parity test updated for step1 heads and a step1 output receipt. | None |
| `uv run tier-a --matrix-only --allow-stale-sut` | NOT RUN: STOP; required cache adapter and immediate preceding build would still be necessary. | None |
| `uv run tier-a --quick --lang ts,js --allow-stale-sut` | NOT RUN: STOP; no same-worktree fresh main/base control. Rust quick remains excluded by static-only authority. | None |

Step0 matrix/quick/doctest/parity/mutation claims are historical and do not validate the union. No full multi-corpus Tier-A, non-JS control, private F run, independent review, or landing was performed. The full suite is not green. The scoped mutant registry additions, executed main-REDs, real public measurements, per-UNDECIDED disclosures and all unrun gates remain required if the owner reopens this lane.

## Controller command and custody

[CONTROLLER.md](CONTROLLER.md) contains the exact provisional step1 `diff` command using reused step0 base binaries and frozen step1 head binaries. Step1 parity was not executed, and the command is not an accepted candidate gate. F was never opened or measured by the planner. The supplied step0 F run retained all 113 targets: 109 WRONG, 0 CORRECT, 4 UNDECIDED, 0 INADMISSIBLE; its narrow-rule removal was zero. Its R-bound census price was 109 WRONG and four UNDECIDED, not a union real build.

No Git writes occurred. Both E5-step1-src.patch and E5-step1-docs.patch are ordinary git-diff patches relative to d01fd65f under step1/. The stopped product and packet are also snapshotted there. Controller owns commit custody; this is a salvageable retained artifact, not a fresh restart, landed change, or completed step.

## User verification-hook extension

The subsequent user verification hook (`stop:5:/Users/wesleyjinks/.codex/hooks.json`) explicitly requested missing tests and root `## Verified` / `## Not verified` sections. One verification extension added two synthetic native-oracle groups without changing product code or existing expectations. Both fail on the exact archived d01fd65f helper (0 pass / 2 fail); the complete current native-oracle suite passes 13/13. Controls include every require initializer shape, variable/parameter/binding-element/function/class locals, unbound/import/destructured require, both union arms, overlap and script/import negatives. All 734 product files and frozen binary hashes remain unchanged, so Rust totals and the readiness STOP stand. Controller parity, main runtime REDs, remaining product gates and real union corpus captures stay unrun. The root verification file is ignored by Git and is explicitly included as a new file in the refreshed docs patch. Evidence is step1/verification-hook/verification-result.json, base-oracle-red-final.tap and head-oracle.tap.
