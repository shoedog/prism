# E5 step2 bounded review fold and re-measurement

**Status: STOP on R1: a valid namespace import-equals with terminal trivia still loses a correct target. Private step2 F and confirming review round 2/2 were not run.** HEAD/patch base ce6de397; main bab0dfbd. Source and packet remain in the working tree; no Git writes. All source/producer claims below are measured this turn unless marked historical/controller-supplied.

## STOP: concrete R1 refutation

WRONG, confidence99/100. `ns.ts` exports namespace N containing function f; `caller.ts` imports N, writes `import f = N./* ordinary trivia */f`, then calls f. Main/base returns ns.ts:f@1-1, Exact free_single; step2 returns LocalBindingUnproven with no target. The collector compares raw text after the dot, including the comment, instead of the terminal identifier. The same-environment main result and unchanged static namespace identity establish the correct loss. Full input/outputs: step2/namespace-alias-trivia-probe/; decision: step2/STOP.json. This is a closed bounded defect: read the AST terminal name while ignoring trivia and add this full-result regression. No implementation or corpus producers ran after STOP. A later verification hook explicitly requested a fresh full suite; its result is recorded below and in VERIFICATION.md. The already-running measurement completed before cancellation was needed. No restart.

## Rulings, tests, mutation pins

| Ruling | Change and test | Authoritative mutant |
|---|---|---|
| R1 | Plain import-require-equals (including recovered exported form), plain same-terminal namespace alias and awaited-import destructuring implemented. R1 remains OPEN: terminal trivia loses a correct binding. Full Exact targets plus renamed/unrelated/invalid twins in e5_import_equals_and_awaited_import_evidence_preserve_fallbacks. Shared extractor unchanged; both T format rows still refused. | E5-09, 10, 11, 14, 15 |
| R2 | Unbound JS/TS/TSX targets outside the refusal set use arm (a) alone. Module-to-script controls in all constructors/grammars; local-alias, parameter, module-target, Source JSX and C-target twins. Parameter protection is earlier-rung UnknownName; local alias exercises E5. | E5-12, 16, 17, 18 |
| R3 | Source-origin conjunct on arm (b); ordinary and renamed JSX indirect targets preserved. Source JSX local alias still refused. Object+property indirect indexing and never offering super to free-function rungs are follow-ups. | E5-13 |
| R4 | Six new r2_d12_w2{m,d}_script_{js,ts,tsx} tests retain full Exact FreeSingle targets and argument edges, then assert no outgoing new-formal reachability. Old variants still assert E5 drops. | PD-29 rebound to the six script variants |
| R5 | Full-result arm-(a) script alias/module-target and script JSX controls; target merge in both directions, serialization, earlier-rung controls labelled. Both arms have controls against over-broad refusal. | E5-01–08 plus R1–R3 mutants |
| R6 | Dead graph-target guard and redundant qualifier/cross-file conjuncts removed after Rust-only call-chain and R4/None-branch proof. Sole remaining predicate call is at final R5. | No killable dead-guard mutant; full non-JS and non-call parity below |
| R7 | Shared is-module helper, cache115 comment, exact ten b5 changes per grammar against main and test rename; duplicate SB counts, measured-zero coverage limit and JSX correction; prospective CLAUDE sentence drafted in spec, CLAUDE unchanged. | Existing cache pins pass suite; no old b5 selectors in registry |

## Measured target removals beside prediction

| Corpus | Prediction W/C/U/I | Measured W/C/U/I | Delta |
|---|---|---|---|
| X | 19 / 0 / 11 / 1 | 19 / 0 / 11 / 1 | 0 in every column |
| T | 201 / 0 / 0 / 2 | 201 / 0 / 0 / 2 | 0 in every column |
| SB | 170 / 0 / 0 / 354 | 170 / 0 / 0 / 354 | 0 in every column |

Step1 → step2 preserves 132 former removals, fully enumerated in step1-to-step2-differences.jsonl: {"SB":{"R3|INADMISSIBLE":64,"R3|WRONG":1},"X":{"R3|INADMISSIBLE":36,"R3|UNDECIDED":23,"R3|WRONG":8}}. Every row is attributed to R1, R2 or R3. Nothing is silently re-baselined.

| Corpus | Arm | W | C | U | I |
|---|---|---:|---:|---:|---:|
| X | a_only | 0 | 0 | 0 | 0 |
| X | b_only | 19 | 0 | 11 | 0 |
| X | both | 0 | 0 | 0 | 1 |
| T | a_only | 0 | 0 | 0 | 0 |
| T | b_only | 148 | 0 | 0 | 0 |
| T | both | 53 | 0 | 0 | 2 |
| SB | a_only | 123 | 0 | 0 | 203 |
| SB | b_only | 44 | 0 | 0 | 29 |
| SB | both | 3 | 0 | 0 | 122 |

| Corpus | Site origin | Physical invocation | W | C | U | I |
|---|---|---|---:|---:|---:|---:|
| SB | Source | jsx | 0 | 0 | 0 | 5 |
| SB | Source | ordinary | 170 | 0 | 0 | 326 |
| SB | Source | qualified | 0 | 0 | 0 | 23 |
| T | Source | ordinary | 201 | 0 | 0 | 2 |
| X | Source | jsx | 5 | 0 | 0 | 0 |
| X | Source | ordinary | 14 | 0 | 11 | 1 |

SecBench: per-root 170 | 0 | 0 | 354; distinct-source 134 | 0 | 0 | 293 (W/C/U/I). 11 of 579 admitted roots contribute a removal, representing 10 distinct contributing sources. The two total.js_3.4.6 roots have identical 97 rows ({"INADMISSIBLE":61,"WRONG":36}). Historical step1: 14 contributing roots, per-root 171 W/418 I, distinct 135 W/357 I.

The corpora contain no import-equals or module-to-script-global rows classified CORRECT. The measured zero does not cover these shapes; new fixtures do. UNDECIDED/INADMISSIBLE are exposure, not yield. Private step1 F is controller-supplied historical evidence: 109 W/0 C/4 U/0 I removed; private step2 is not run here.

## All call/graph and consumer deltas

758 differing call sites, 758 targets and 1938 graph rows, each adjudicated. Added sites/targets/graph rows: 0/0/0. Non-JS serialized calls: 43953 byte-identical rows. Non-call digests: 581/581 identical. Symbol/file inventories identical across all 581 admitted roots. Navigation and module dependencies: {"file_inventory_byte_identical":true,"functions_byte_identical":true,"removed_call_derived_module_edges":177,"roots":581,"sampled_navigation_removed_items":{"callees":20,"callees_unresolved_placeholders":20,"callers":127}}. These query deltas are samples; call/graph censuses are exhaustive over admitted roots. Unresolved placeholders are separate from resolved relationships.

| Corpus | Graph kind | W | C | U | I |
|---|---|---:|---:|---:|---:|
| SB | Call | 170 | 0 | 0 | 354 |
| SB | DataFlow | 162 | 0 | 0 | 144 |
| SB | Return | 170 | 0 | 0 | 354 |
| SB | ReturnContains | 0 | 6 | 0 | 0 |
| SB | ReturnFlow | 11 | 0 | 0 | 9 |
| SB | ReturnInput | 0 | 8 | 0 | 0 |
| T | Call | 201 | 0 | 0 | 2 |
| T | DataFlow | 64 | 0 | 0 | 0 |
| T | Return | 201 | 0 | 0 | 2 |
| T | ReturnFlow | 5 | 0 | 0 | 0 |
| X | Call | 19 | 0 | 11 | 1 |
| X | DataFlow | 5 | 0 | 7 | 0 |
| X | Return | 19 | 0 | 11 | 1 |
| X | ReturnFlow | 1 | 0 | 0 | 0 |

CORRECT ReturnContains/ReturnInput removals are disclosed consumer effects, not CORRECT Exact-target losses. Every differing call site, target and graph row is in step2/union-call-site-adjudications.jsonl, union-target-adjudications.jsonl and graphs/*/graph.adjudicated.jsonl. Every UNDECIDED removal is listed with binding/reason and samples in UNDECIDED-step2.md.

## Reviewer fixtures

93 fixtures / 279 admitted frozen CLI invocations, including all cited fixtures and new negative twins. All 20 required correct-preservation controls retain their complete targets. Base, step1 and step2 bytes/commands/full targets/spans/origin/drop values are retained in step2/reviewer-fixtures.json and step2/reviewer-fixtures/. The complete table follows in REVIEWER-FIXTURES-step2.md.

## Gates and exact commands

| Gate | Result | Command / receipt |
|---|---|---|
| Format | PASS | `cargo fmt --check` → step2/fmt.log |
| Earlier full serial suite | 5309 run, 5308 passed, 1 inherited example failed, 1 skipped; MCP readiness PASS | `cargo nextest run --offline --all-targets --features mcp --no-fail-fast` → step2/full-suite.log |
| Same-worktree main example control | Same complete later/empty-bindings mismatch, 1 test failed | `cargo nextest run --offline --features mcp --all-targets -E "binary(parameter_slot_characterization) and test(d7_predicate_and_singular_parameter_rows_are_complete)"` on exact main bytes → main-example-control.log; restored hashes in quick-controls.json |
| Doctests | 2/2 PASS | `cargo test --doc --offline --features mcp` → step2/doctests.log |
| Scoped authoritative mutations | All 19/19 KILLED, 19/19 admissible: every E5 mutant + PD-29 | `python3 /Users/wesleyjinks/prism-evidence/js-e5/step2/mutation.py`; exact driver arguments in mutant-command.json; results in mutants/summary.json |
| Release + frozen helpers | PASS, source/binary hashes bound | `python3 /Users/wesleyjinks/prism-evidence/js-e5/step2/freeze.py`; cargo command below; binary-binding.json |
| Tier-A matrix | 178/178 PASS | cache-bound adapted command below; tier-a-matrix.log |
| Fresh TS/JS quick with same-worktree base | X and SecBench-node VALID on both; M1/M2/M3, failures/pending/summary identical | `python3 /Users/wesleyjinks/prism-evidence/js-e5/step2/quick-controls-2.py`; exact tier commands below; quick-controls.json / quick-comparison.json |
| Worker/controller parity + census tests | 14/14 PASS | `node --test docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-e5.test.cjs docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-step1.test.cjs docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/controller-parity.test.cjs` → controller-tests.log / controller-parity.json |

Exact authoritative mutation command:

```bash
python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --authoritative --jobs 1 --out /Users/wesleyjinks/prism-evidence/js-e5/step2/mutants --only PD-29-cross-file-free-single-predicate-off --only E5-01-module-caller-arm-off --only E5-02-script-module-boundary-off --only E5-03-import-require-exemption-off --only E5-04-jsx-arm-off --only E5-05-target-syntax-arm-off --only E5-06-arm-a-unbound-conjunct-off --only E5-07-arm-a-jsx-conjunct-off --only E5-08-merge-target-refusals-off --only E5-09-import-require-equals-evidence-off --only E5-10-namespace-equals-evidence-off --only E5-11-awaited-import-evidence-off --only E5-12-unbound-script-exception-off --only E5-13-module-origin-conjunct-off --only E5-14-export-require-recovery-off --only E5-15-namespace-terminal-equality-off --only E5-16-unbound-exception-local-conjunct-off --only E5-17-unbound-exception-language-conjunct-off --only E5-18-unbound-exception-target-conjunct-off
```

Exact release/helper build (temporary example copies are source-bound and removed after freeze):

```bash
cargo build --release --offline --bin prism --example e5_step2_facts --example e5_step2_graph --example e5_step2_modules --example e5_step2_bytes
```

Exact matrix / TS-JS quick commands (existing installed Python environment, cache-binding adapter; not represented as unmodified uv commands):

```bash
E5_SUT_BIN=/Users/wesleyjinks/prism-evidence/js-e5/step2/bin/base-prism-fresh PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval /Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --quick --lang ts,js --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step2/tier-a-quick-base
E5_SUT_BIN=/Users/wesleyjinks/prism-evidence/js-e5/step2/bin/head-prism-fresh PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval /Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step2/tier-a-matrix
E5_SUT_BIN=/Users/wesleyjinks/prism-evidence/js-e5/step2/bin/head-prism-fresh PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval /Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --quick --lang ts,js --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step2/tier-a-quick-head
```

Each quick side had an immediately preceding fresh release build in this worktree; head matrix/quick share the immediately rebuilt head. Exact main source is restored in finally and every head source byte checked by hash.

## Controller exact private-corpus command

Set CORPUS_F_ROOT in the controller environment first. The destination below must not exist.

```bash
CORPUS_F_ROOT="$CORPUS_F_ROOT" \
PRIVATE_EVIDENCE_ROOT="$HOME/prism-evidence/js-e5/f-e5-step2-ce6de397-4b6a0e1c" \
bash /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/CONTROLLER-e5.sh diff \
  /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step2/bin/head-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-facts-v2 \
  /Users/wesleyjinks/prism-evidence/js-e5/step2/bin/head-facts \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-graph \
  /Users/wesleyjinks/prism-evidence/js-e5/step2/bin/head-graph
```

## Exclusions, STOP and custody

Not verified: private step2 F; confirming independent review round 2/2; live MCP refresh; full human-triggered Tier-A multi-corpus/Rust quick; full unscoped mutation lane; landing/commit/push. Rust quick is outside this TS/JS gate and may execute corpus build scripts/proc macros. No corpus packages were executed. Cache version and both registry pins remain115; CLAUDE.md is unchanged.

The skipped test is resolution_test::slice_elem_variant_reserved, ignored because SliceElem is reserved until a future slice (tests/integration/resolution_test.rs:575). Four original SecBench producer exclusions are paired and receive no zero-yield/zero-cost credit:
- `SB__path-traversal__atropa-ide_0.2.2-2`: paired exclusion: timeout 1500s
- `SB__redos__clean-css_4.1.10`: paired exclusion: exit -9
- `SB__redos__natural_5.1.0`: paired exclusion: timeout 1500s
- `SB__redos__three_0.122.0`: paired exclusion: timeout 1500s

STOP: R1 correct namespace import-equals loss on the synthetic terminal-trivia fixture. Source left unchanged after STOP. The inherited example failure is reported, not fixed or re-baselined. Private step2 and confirming review remain explicit adoption gates. Source/packet snapshots and patches are relative to ce6de397 in step2/E5-step2-src.patch and E5-step2-docs.patch. Final patch apply checks, hashes and custody are in step2/patch-manifest.json and final-custody.json.

Additional audit fixture: | Namespace terminal trivia | Main Exact free_single ns.ts:f@1 | Step1 NOT RUN after STOP | Step2 drop LocalBindingUnproven — WRONG / STOP |


## Verification hook stop:5 continuation

The hook explicitly authorized verification/documentation after STOP; implementation remains unchanged and R1 remains open. Exact command: `cargo nextest run --offline --all-targets --features mcp --no-fail-fast`. Fresh result:5309 run,5307 passed,2 failed,1 ignored. The failures are the inherited d7 characterization example (same-environment main control already recorded) and mcp::lazy::tests::queued_calls_share_one_cumulative_wait_budget at its400ms elapsed assertion. The latter is outside the E5 edit scope and remains unattributed; no same-environment base/full-suite control was run for it.

Five isolated head runs of `cargo nextest run --offline --features mcp --lib -E 'test(mcp::lazy::tests::queued_calls_share_one_cumulative_wait_budget)'` each ran one test and passed. This does not distinguish scheduling pressure from shared-state interference, does not erase the full-suite failure, and does not certify a source regression or inherited failure. The declared cap was five; no whole-suite retry, code repair, timeout change, ignore or re-baseline followed.

Receipts: step2/verification-hook-stop5/full-suite-result.json, full-suite.log and mcp-timing-alone.json. Root VERIFICATION.md has exact commands, regression/negative-case coverage, Verified/Not verified and explicit exclusions. The file is ignored by Git; its bytes are explicitly included in the refreshed docs patch and snapshot.
