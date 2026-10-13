# E5 step2b — bounded R1 trivia closure

**Status: planner step2/step2b complete; no STOP. Private F and confirming independent review round2/2 remain controller adoption gates.** Patch base 3df0ddfa88ed94033ef5ec8a1813db0b3d815bd4; step2 source01798466 / packet3df0ddfa; frozen mainbab0dfbd (local main has advanced to c8de720b and is not the control). Changes remain in the working tree; no Git writes.

## Fix, alternatives and full class audit

Read the namespace alias terminal from its AST property/identifier token instead of raw suffix text. Filter comment children before selecting import-alias local/entity, direct awaited-import operands and exported-require recovered strings. Exported recovery still requires a missing semicolon and exactly one string semantic argument. The binder import_alias local also selects a direct identifier: the collector-only fix populated f but still left the call Unproven(unbound), triggering unchanged arm(a). The same-environment base kept the complete target, and binder-diagnostic.log rules out the alternative import_parse_recovery drop.

Neither E5 predicate, shared extract_imports, nor any ruling changed. Unsupported parentheses, .then and type-assertion wrappers are explicit negative cases, not new import evidence. All audited places, robust/fixed classifications, mechanism proofs and fixture names: [CLASS-AUDIT-step2b.md](CLASS-AUDIT-step2b.md).

Full-result test e5_r1_trivia_preserves_covered_shapes_and_refuses_twins has68 rows x3 constructors =204 cells. Pre-change source failed28 rows/84 cells; all failures enumerated before retry. Corrected head passes all cells, requiring file/name/start/end/kind/confidence/no-drop on preservation cells and empty targets/LocalBindingUnproven on twins. e5_target_syntax_and_identifier_text_are_trivia_robust adds24 plain/commented target pairs. E5-15's patch still applies; its selectors now include the new complete table. The original terminal-trivia reviewer fixture preserves ns.ts:f@1-1 Exact free_single on step2b. Some conservative evidence fixtures intentionally assert retained legacy fallback, not proof of positive target identity.

Probe log and source-bound RED/diagnostic/GREEN artifacts: step2b/PROBE-LOG.md, pre-change-red.log, red-failure-population.json, binder-diagnostic.log, post-change-focused-2.log and full-suite.log. The first whole-suite compile attempt is INADMISSIBLE (test closure lifetime error, zero tests); corrected before the one admitted full run. No behavioral conclusion is drawn from that failed probe.

## Head-to-head identity and carry-over

| Corpus | Admitted roots | Differing call rows | Differing graph rows |
|---|---:|---:|---:|
| X | 1 | 0 | 0 |
| T | 1 | 0 | 0 |
| SB | 579 | 0 | 0 |

All581 roots are complete, with zero removed/added rows in both comparisons. Compared 422095 call-output rows and 3628510 graph/digest rows. Call files byte-identical on 581/581 roots; graph files byte-identical on 581/581. Every new CLI run uses a unique step2b cache; helpers build directly without a disk cache. Exact commands, input/output hashes and per-root differences: step2b/head-identity.json, identity-rows/ and head-identity-progress.jsonl. Four producer slots maximum; no test/build overlaps.

The step2 tables and adjudications carry over unchanged, rather than being re-adjudicated. Target removals W/C/U/I: X19/0/11/1, T201/0/0/2, SB170/0/0/354, all equal prediction. Distinct-source SB134/0/0/293;11 of579 roots contribute,10 distinct contributing sources; two total.js roots duplicate97 rows (36W/61I). The758 target/site and1938 graph adjudications,43953 non-JS call rows, all581 non-call digests and inventories, arm/origin/invocation/navigation/dependency tables and UNDECIDED list remain exactly the step2 evidence. No fresh oracle adjudication, inventory query or navigation re-capture is claimed. Zero corpus difference transfers those source-bound results; it does not independently repeat them. Corpora have no classified-CORRECT import-equals/module-to-script rows, so synthetic controls provide those coverage shapes.

## Four-way reviewer fixtures

162 fixtures /648 fresh frozen CLI invocations: all93 prior fixtures,68 new audit cases and the original STOP fixture. All20 original designated correct-preservation controls hold, and the original STOP target is restored. All68 new cases meet complete head expected values; 53 total baseline/policy preservation controls were checked. Base/step1/step2/step2b table: [REVIEWER-FIXTURES-step2b.md](REVIEWER-FIXTURES-step2b.md); complete rows/commands/hashes in step2b/reviewer-fixtures.json and reviewer-fixtures/. No packages executed.

## Gates and exact commands

| Gate | Result | Command / evidence |
|---|---|---|
| Format | PASS | `cargo fmt --check`; fmt.log |
| Full serial suite |5311 run,5310 pass,1 inherited failure,1 skipped;3 slow,1 leaky | `cargo nextest run --offline --all-targets --features mcp --no-fail-fast`; full-suite.log/full-suite-result.json |
| Same-environment base example control |1 run,0 pass,1 failure; complete assertion values identical | `cargo nextest run --offline --features mcp --all-targets -E 'binary(parameter_slot_characterization) and test(d7_predicate_and_singular_parameter_rows_are_complete)'` on exact bab0dfbd bytes; example-control.json/main-example-control.log |
| Doctests |2/2 PASS | `cargo test --doc --offline --features mcp`; doctests.log |
| Authoritative scoped mutation |19/19 KILLED,19/19 admissible, every E5 + PD-29 | Exact command below; mutants/summary.json |
| Release and frozen helpers |PASS; five frozen binary hashes and full source binding | Exact command below; binary-binding.json/release-freeze.log |
| Tier-A matrix |178/178 ok verdicts | Adapted exact command below; tier-a-matrix.log |
| Same-worktree base/head TS/JS quick |X and SecBench-node VALID on both; M1/M2/M3,pending,failures,summary identical | Commands below; quick-controls.json/quick-comparison.json |
| Actual-shell controller parity + census tests |14/14 PASS | Exact command below; controller-tests.log/controller-parity.json |

The inherited failure is parameter_slot_characterization::tests::d7_predicate_and_singular_parameter_rows_are_complete: actual bare-arrow later binding versus empty expected bindings. Both complete left/right assertion values match fresh exact-main same-worktree control. No re-baseline or repair. The skipped test is resolution_test::slice_elem_variant_reserved (future SliceElem slice). Nextest also reports one passing test as leaky: navigation cache_test::grammar_fingerprint_is_present;3 tests slow. These observations are reported, not attributed to E5. Both queued_calls_share_one_cumulative_wait_budget and lazy_runtime_delegates_each_refresh_policy_after_readiness pass in this new full suite. The prior step2 queued timing fluctuation remains historical/unattributed; no isolated timing run was required or performed on this head.

```bash
python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --authoritative --jobs 1 --out /Users/wesleyjinks/prism-evidence/js-e5/step2b/mutants --only PD-29-cross-file-free-single-predicate-off --only E5-01-module-caller-arm-off --only E5-02-script-module-boundary-off --only E5-03-import-require-exemption-off --only E5-04-jsx-arm-off --only E5-05-target-syntax-arm-off --only E5-06-arm-a-unbound-conjunct-off --only E5-07-arm-a-jsx-conjunct-off --only E5-08-merge-target-refusals-off --only E5-09-import-require-equals-evidence-off --only E5-10-namespace-equals-evidence-off --only E5-11-awaited-import-evidence-off --only E5-12-unbound-script-exception-off --only E5-13-module-origin-conjunct-off --only E5-14-export-require-recovery-off --only E5-15-namespace-terminal-equality-off --only E5-16-unbound-exception-local-conjunct-off --only E5-17-unbound-exception-language-conjunct-off --only E5-18-unbound-exception-target-conjunct-off
cargo build --release --offline --bin prism --example e5_step2b_facts --example e5_step2b_graph --example e5_step2b_modules --example e5_step2b_bytes
node --test /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-e5.test.cjs /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-step1.test.cjs /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/controller-parity.test.cjs
```

Temporary helper sources were compiled/frozen and removed by step2b/freeze.py. Each quick side had an immediate same-worktree release rebuild; exact base was installed only for its example/quick control and all head source bytes were restored by hash. Matrix/head quick share the immediately rebuilt head. The installed Python environment and fresh-cache adapter replace the unmodified uv/SUT commands and are disclosed explicitly. Initial base quick exhausted the default600s cumulative SUT/oracle budget on X (12 SUT probe timeouts plus the final syntax census), while SecBench-node was valid. It is preserved as INADMISSIBLE in quick-attempt1/quick-inadmissible.json; no source conclusion is drawn. Both final quick sides use the supported --oracle-budget-s2400 with unchanged sample3 and immediate rebuilds:

```bash
E5_SUT_BIN=/Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/base-prism-fresh PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval /Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --quick --lang ts,js --oracle-budget-s 2400 --allow-stale-sut --sut-bin /Users/wesleyjinks/prism-evidence/js-e5/step2b/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step2b/tier-a-quick-base
E5_SUT_BIN=/Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-prism-fresh PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval /Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin /Users/wesleyjinks/prism-evidence/js-e5/step2b/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step2b/tier-a-matrix
E5_SUT_BIN=/Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-prism-fresh PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval /Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --quick --lang ts,js --oracle-budget-s 2400 --allow-stale-sut --sut-bin /Users/wesleyjinks/prism-evidence/js-e5/step2b/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step2b/tier-a-quick-head
```

## Controller exact private-corpus command

Set CORPUS_F_ROOT in the controller environment first. The evidence destination must not exist. Only the controller executes this private capture.

```bash
CORPUS_F_ROOT="$CORPUS_F_ROOT" \
PRIVATE_EVIDENCE_ROOT="$HOME/prism-evidence/js-e5/f-e5-step2b-3df0ddfa-a2b4841d" \
bash /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/CONTROLLER-e5.sh diff \
  /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-facts-v2 \
  /Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-facts \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-graph \
  /Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-graph
```

## Exclusions, STOP and custody

No current STOP. Not verified: private step2b F; independent confirming review round2/2 (not dispatched by this planner); live MCP refresh; full human-triggered Tier-A multi-corpus/Rust quick; full unscoped mutation lane; commit/push/landing. No provider/network calls, Git writes, private-root inspection or corpus package execution. CLAUDE.md unchanged; cache/pins115 and navigation62 unchanged. No adoption or whole-feature completion is claimed.

Four original paired SecBench producer exclusions remain excluded, with no zero-yield/zero-cost credit:
- SB__path-traversal__atropa-ide_0.2.2-2: timeout1500s
- SB__redos__clean-css_4.1.10: exit-9
- SB__redos__natural_5.1.0: timeout1500s
- SB__redos__three_0.122.0: timeout1500s

Source/docs patches relative to3df0ddfa: step2b/E5-step2b-src.patch and E5-step2b-docs.patch. Source/binary manifest, final snapshot and patch validation are in step2b/binary-binding.json, final-source-packet.tar.gz, patch-manifest.json and final-custody.json. Steps0–2 were read-only. Historical packet statuses are explicitly superseded; current state is SPEC-e5.md/REPORT-step2b.md/HANDOFF-step2b.md. Root VERIFICATION.md is ignored by Git and explicitly included in the docs patch.

## Verification hook stop:5 continuation

On unchanged source, a fresh `cargo nextest run --offline --all-targets --features mcp --no-fail-fast` ran5311 tests:5310 passed,1 inherited d7 example failed,1 skipped (3 slow). Complete assertion left/right values freshly match the retained exact-main same-environment control. Both MCP timing tests pass; no isolated run or code repair followed. Coverage inspection found no missing RED/negative-case regression; no source/test changes. Root VERIFICATION.md now has explicit Verified and Not verified sections, exact commands/totals and exclusions. Receipts: step2b/verification-hook-stop5/{full-suite.log,full-suite-result.json,coverage.json}. No current STOP; private F/independent review remain controller gates.
