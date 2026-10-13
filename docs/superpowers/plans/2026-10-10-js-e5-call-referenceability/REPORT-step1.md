**Superseded for current operational state by HANDOFF-step2.md and REPORT-step2.md. This document records the historical step1 head and its receipts.**

# E5 Step1 complete — measured owner-selected union

**Status: PLANNER COMPLETE / READY FOR CONTROLLER F AND SPEC REVIEW; not adopted or landed. No current STOP.** The step1b controller rulings reopened the preserved artifact. Repair round1 of cap2 passed; no extension or restart.

## Real paired public targets

| Corpus | WRONG removed | CORRECT lost | UNDECIDED removed | INADMISSIBLE removed |
|---|---|---|---|---|
| X | 27 | 0 | 34 | 37 |
| T | 201 | 0 | 0 | 2 |
| SB | 171 | 0 | 0 | 418 |

| Corpus | Arm | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | a_only | 0 | 0 | 0 | 0 |
| X | b_only | 27 | 0 | 34 | 36 |
| X | both | 0 | 0 | 0 | 1 |
| T | a_only | 0 | 0 | 0 | 0 |
| T | b_only | 148 | 0 | 0 | 0 |
| T | both | 53 | 0 | 0 | 2 |
| SB | a_only | 123 | 0 | 0 | 203 |
| SB | b_only | 45 | 0 | 0 | 93 |
| SB | both | 3 | 0 | 0 | 122 |

## Arm(b) physical invocation split

| Corpus | Arm(b) invocation | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | ordinary | 14 | 0 | 11 | 1 |
| X | jsx | 13 | 0 | 23 | 35 |
| X | qualified | 0 | 0 | 0 | 1 |
| T | ordinary | 201 | 0 | 0 | 2 |
| T | jsx | 0 | 0 | 0 | 0 |
| T | qualified | 0 | 0 | 0 | 0 |
| SB | ordinary | 47 | 0 | 0 | 123 |
| SB | jsx | 0 | 0 | 0 | 5 |
| SB | qualified | 1 | 0 | 0 | 87 |

Physical JSX is classified by the native invocation node, not the internal jsx flag:66 JSX spans are carried by IndirectResolution records with jsx=false. The table separately discloses89 qualified physical spans (1 W/88 I);66 are indirect and23 Source records have no retained product qualifier. The unchanged real R-bound guard acts on qualifier-less product call sites; no blanket physical-qualified preservation claim is made. Every ordinary row in this measured b population has Source origin: the real R-bound-source subset is X14 W/0 C/11 U/1 I, T201 W/0 C/0 U/2 I, SB47 W/0 C/0 U/123 I. R-bound minus R-bound-source therefore includes both physical JSX and qualified fallback records.

All34 UNDECIDED removals are individually described in [UNDECIDED-step1.md](UNDECIDED-step1.md), with up to ten public samples. They receive no WRONG yield credit; the signature/value-binding uncertainty and local declarations are retained per target. INADMISSIBLE removals remain unknown oracle cost. Raw/after-override agree; no overrides.

## Every differing consumer and parity

| Corpus | Removed row kind | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | Call | 27 | 0 | 34 | 37 |
| X | DataFlow | 5 | 0 | 7 | 0 |
| X | Return | 27 | 0 | 34 | 37 |
| X | ReturnFlow | 1 | 0 | 0 | 0 |
| T | Call | 201 | 0 | 0 | 2 |
| T | DataFlow | 64 | 0 | 0 | 0 |
| T | Return | 201 | 0 | 0 | 2 |
| T | ReturnFlow | 5 | 0 | 0 | 0 |
| SB | Call | 170 | 0 | 2 | 417 |
| SB | DataFlow | 162 | 0 | 0 | 144 |
| SB | Return | 170 | 0 | 2 | 417 |
| SB | ReturnContains | 0 | 7 | 0 | 0 |
| SB | ReturnFlow | 11 | 0 | 0 | 11 |
| SB | ReturnInput | 0 | 11 | 0 | 0 |

All581 call-site and graph comparisons are admitted. Every differing call site (890) and graph row (2208) is classed and adjudicated in `union-call-site-adjudications.jsonl` / each graph.adjudicated.jsonl. Added targets/sites/graph rows:0/0/0. All43953 original serialized non-JS call rows are byte-identical; all581 non-call digests match. All581 symbol/file inventories match; 225 call-derived module dependencies are removed. Sampled caller/callee queries cover affected roots; aggregate deltas are `{"callees": 27, "callees_unresolved_placeholders": 27, "callers": 148}`. Unresolved source placeholders are disclosed separately from resolved relationships. Full receipts retain every inventory/query command and any reused base artifact.

## Existing fixtures and every deliberate expectation change

| Existing test | Fixture edit | Purpose retained |
|---|---|---|
| `td_duplicate_for_heads_negative_control_single_head_binds` | Caller `export function run` → script `function run`; target was already a script | `L2 f: Exact free_single f@1-1` unchanged; negative duplicate/for-head scope control remains reachable outside module refusal. |
| `rep_unbound_name_through_an_unrelated_parameter_keeps_base_at_r5` | Caller `export function run` → script `function run`; helper/target unchanged scripts | `L5 f: Exact free_single f@1-3` unchanged; unrelated helper parameter remains outside caller scope. |
| `c41_with_reason_keeps_base_at_r5` | Caller and target declarations lose `export` | `L3 f: Exact free_single f@1-3` unchanged; `Unproven("with")` still proves the older R5 drop is reason-scoped. |

| b5 scenario (both `.jsx` / `.tsx`) | Deliberately changed result | Reason |
|---|---|---|
| Bare nested `const { f }` block, L4 | `Exact free_single lib:f@1-3` → `drop LocalBindingUnproven` | Module caller has no import/require evidence, even for in-scope alias. |
| Nested `if` / `let { f }`, L4 | Same Exact → drop | Same module-caller alias policy decision. |
| Nested `var { f }` block, L4 | Same Exact → drop | Union overrides unsupported nested var alias fallback. |
| Nested `var { f }` block, L6 | Same Exact → drop | Union overrides unsupported hoisted var alias fallback. |
| Same-line nested const block, first L2 call | Same Exact → drop | Module alias lacks import/require evidence; the second L2 call was already drop. |
| Module-scope nested var block, L5 | Same Exact → drop | Module-level var fallback lacks import/require evidence. |

All already-refused expectations stay unchanged. Imported b5 controls stay at complete values `L5 f: Exact import_member lib:f@1-3` and `L7 f: Exact import_member lib:f@1-3` in both grammars. Six scenario values change (twelve across the two grammars); the bare/if cases share one expectation vector. No assertion is weakened.

## R-local

| Predicate (base census only) | Corpus | WRONG | CORRECT | UNDECIDED | INADMISSIBLE | Literal scalar-require C costs |
|---|---|---|---|---|---|---|
| R-local | X | 19 | 0 | 11 | 0 | 0 |
| R-local | T | 148 | 0 | 0 | 0 | 0 |
| R-local | SB | 1249 | 10 | 17 | 24 | 10 |
| R-local-no-require | X | 19 | 0 | 11 | 0 | 0 |
| R-local-no-require | T | 148 | 0 | 0 | 0 | 0 |
| R-local-no-require | SB | 449 | 0 | 2 | 24 | 0 |

**Accepted disposition: follow-up, outside this slice.** Naive axis-based R-local costs ten CORRECT literal scalar-require bindings. Exempting every require-initialized local has zero classified CORRECT cost and retains 616 WRONG (19/148/449), plus 13 UNDECIDED and 24 INADMISSIBLE exposures. These are whole-base census prices, not marginal post-union yield. Both predicates remain in controller summaries; no runtime R-local work was added in Step1b.

## Gates and exact commands

| Exact command | Result | Evidence under step1/ |
|---|---|---|
| `cargo nextest run --offline --all-targets --features mcp --no-fail-fast` | 5298 run; 5297 passed; 1 explicitly excluded inherited failure; 1 reserved skip; 233.399s | step1b-full-tests-round1.log |
| `CARGO_TARGET_DIR=/Users/wesleyjinks/code/prism-e5-plan/target cargo test --offline --lib resolution::e5_ -- --nocapture` (cwd main-red-source) | Exact main bab0dfbd: 10 selected, 3 preservation PASS, 7 refusal/lifecycle/consumer RED; same ten head groups PASS in full suite | main-red-final.log; main-red-binding.json |
| `cargo fmt --check` | PASS | step1b-fmt.log |
| `cargo test --doc --offline --features mcp` | 2/2 PASS | step1b-doctests.log |
| `python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --since d01fd65f --scope fn --authoritative --jobs 1 --out /Users/wesleyjinks/prism-evidence/js-e5/step1/mutants` | 4/4 selected, killed and admissible; 3 green baseline selectors | step1b-mutation.log |
| `node --test /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-e5.test.cjs /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/census-step1.test.cjs /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/controller-parity.test.cjs` | 14/14 PASS, including actual-shell worker/controller parity | step1b-native-controller.log |
| `cargo build --release --offline --bin prism --example e5_step1_modules --example e5_step1_byte` | PASS, 38.01s | step1b-release.log |
| `/Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step1/tier-a-matrix` | 178/178 actual ok rows | step1b-tier-matrix.log |
| `cargo build --release --offline --bin prism` | PASS; same-worktree source control | step1b-base-fresh-build.log |
| `/Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --quick --lang ts,js --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step1/tier-a-quick-base-fresh` | PASS; immediate release then TS/JS quick; both corpora VALID | step1b-base-fresh-quick.log |
| `cargo build --release --offline --bin prism` | PASS; same-worktree source control | step1b-head-fresh-build.log |
| `/Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python -m tier_a.cli --quick --lang ts,js --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/probes/prism-bound.py --out-dir /Users/wesleyjinks/prism-evidence/js-e5/step1/tier-a-quick-head-fresh` | PASS; immediate release then TS/JS quick; both corpora VALID | step1b-head-fresh-quick.log |

Tier-A uses the installed read-only Python environment and this worktree's eval package, with `PYTHONPATH=/Users/wesleyjinks/code/prism-e5-plan/eval` and `E5_SUT_BIN` set to the immediately rebuilt immutable binary. Matrix uses `step1/bin/head-prism-gate`; fresh quick uses `step1/bin/base-prism-fresh-step1b` then `head-prism-fresh-step1b`. The `prism-bound.py` adapter binds every CLI to the E5 cache and replaces the harness's no-cache default. These are adapted commands, not a claim that bare `uv run` was executed; eval/.venv is absent, so the installed environment is reused without installation. Mutation scope starts at d01fd65f to select the current predicate function; delivery patches start at d994c75d. Jobs1 serializes mutant build/test.

| Corpus | Base valid | Head valid | m1 | m2 | m3 | pending | failures | summary |
|---|---|---|---|---|---|---|---|---|
| excalidraw-ts | True | True | True | True | True | True | True | True |
| secbench-node | True | True | True | True | True | True | True | True |

The excluded example selects1/fails1 on exact main in this same environment with equal head assertion payload: `const take = later => later;` yields the `later` binding at bytes13–18 while its fixture expects no bindings. It remains unchanged and is excluded from union acceptance. The MCP readiness test passed in the serial head suite, so isolated five-run retries were not needed.

## Binary/input custody and exclusions

Entry and patch base d994c75d; preserved union source e164178b. Product src/vendor bytes remain identical to the frozen union build; Step1b changes four integration fixtures/expectations and the lane registry, plus measurement/packet tools. CACHE_VERSION and both pins stay115; D12 and navigation cache62 are unchanged. Public and controller captures use original frozen step1/bin/head-prism, head-facts, head-graph; hashes are verified in binary-binding.json and step1b-frozen-head-reuse.json. Base calls/facts/graphs and admitted matching-command base navigation artifacts reuse read-only step0 captures. Base binaries40ad8390 equal main bab0dfbd product source: supplied controller control and the freshly verified empty full Git diff across src/Cargo/build/vendor (base-main-product-equality-step1b.json). New module/byte helpers and gate binary are frozen in step1b-helper-source-binding.json. Fresh quick builds exact main and restored head in this same worktree, immediately before each quick; every modified source byte is restored by hash. No product source changed, so original public/controller head binaries require no re-freeze.

| Excluded SB root | Reason | Original base wall time(s) | Head disposition |
|---|---|---|---|
| SB__path-traversal__atropa-ide_0.2.2-2 | timeout 1500s | 1500.021 | Paired exclusion; no new producer or zero-cost observation |
| SB__redos__clean-css_4.1.10 | exit -9 | 394.508 | Paired exclusion; no new producer or zero-cost observation |
| SB__redos__natural_5.1.0 | timeout 1500s | 1500.931 | Paired exclusion; no new producer or zero-cost observation |
| SB__redos__three_0.122.0 | timeout 1500s | 1501.928 | Paired exclusion; no new producer or zero-cost observation |

## Controller F command

Worker/controller parity passed on the original frozen Step1 head binaries; the command below is ready for the controller. The planner did not run F. Historical F base and narrow head both retained113 targets:109 WRONG/0 CORRECT/4 UNDECIDED/0 INADMISSIBLE, zero removals. Historical R-bound price reaches109 W/4 U; it is not a union real measurement.

```bash
CORPUS_F_ROOT="/path/to/private/root" \
PRIVATE_EVIDENCE_ROOT="/path/to/new/private/evidence" \
bash /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/CONTROLLER-e5.sh diff \
  /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step1/bin/head-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-facts-v2 \
  /Users/wesleyjinks/prism-evidence/js-e5/step1/bin/head-facts \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-graph \
  /Users/wesleyjinks/prism-evidence/js-e5/step1/bin/head-graph
```

## Not verified

Private F union capture; controller Opus/gpt-6.1-sol spec review (cap two rounds); adoption/landing/Git custody; R-local runtime; D12 narrowing or incoming-formal repair; the four excluded SecBench roots. Full human-triggered Tier-A multi-corpus and Rust quick were not run. Rust quick is excluded because rust-analyzer build scripts/proc macros can execute corpus code and write outside granted roots. No provider/network calls, corpus package execution, Git writes or directory named frontend-portal were used.
