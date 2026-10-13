**Current status:** Step2b closes the R1 trivia class with gates and zero differing call/graph rows on all581 admitted roots. Step2 tables/adjudications carry unchanged. See SPEC-e5.md, REPORT-step2b.md and HANDOFF-step2b.md. Earlier commands/receipts below are historical. Private F and confirming review round2/2 remain controller work.

# Prototype validation and exclusions

Product source relative to base40ad8390 is in E5-src.patch. This is uncommitted planning work; no Git writes, provider/network calls or corpus execution occurred. No independent review was dispatched. Diagnostic repair cap was two per probe; the prior 30-binary suite cap was three complete head runs. The complete 15-failure population and then the b5-only scope population were enumerated before repairs; the final cap run passed. The stop hook required one disclosed verification extension with explicit all-target selection. That run added34 example tests and found one inherited assertion failure, confirmed on base and left unchanged. The full suite is therefore not green; root VERIFICATION.md is the current gate record.

| Check | Base/control | Recommended head | Evidence in step0 |
|---|---|---|---|
| Prior matched Rust suite,30 binaries |5254 passed /1 skipped |5259 passed /0 failed /1 skipped |base-full-tests.log; head-full-tests-3.log |
| Full Rust all-target suite,35 binaries, mcp, offline |same failing example assertion reproduced |5292 passed /1 failed /1 skipped;5293 run |verification-hook-full-tests.log; verification-hook-base-example.log |
| New declaration syntax regression groups |valid RED:2 failures,2 controls pass |4/4 before consumer addition |e5-red-valid.log |
| New argument/return consumer regression |valid RED:1 failure, observed [1,1,1,1,0] |all five groups pass |e5-red-consumers.log; e5-green-consumers.log |
| Release build |immutable base binary |fresh build and immutable head quick binary |base/binding.json; head/binding.json; head-fresh-build.log; fresh-quick-control.receipt.json |
| Cache-bound adapted Tier-A matrix |matrix controls inherent |178/178 ok |head-tier-matrix-2.log |
| Cache-bound adapted TS/JS Tier-A quick |X and SecBench-node VALID |X and SecBench-node VALID; identical m1/m2/m3/pending/failures/summary |base-tier-quick-fresh.log; head-tier-quick-fresh.log; tier-a-quick-fresh-comparison.json |
| Doctests (mcp, offline) |not needed for regression attribution |2 passed /0 failed |head-doctests.log; verification-hook-doctests.log |
| Default-feature cargo check (offline) |not needed for regression attribution |passes |head-default-check.log |
| Strict clippy, all targets, mcp, -D warnings |fails |fails; identical diagnostic multiset,46 distinct messages |base-clippy.log; head-clippy.log |
| Primary native oracle and actual controller parity |synthetic public source controls |12 passed /0 failed, repeated at hook |oracle-controller-final.tap; verification-hook-oracle-controller.tap; controller-parity-v5.json |
| Formatting |not attributed as a regression |cargo fmt --check passes |final-fmt-check.log |
| Patch hygiene |40ad8390 archive control |git diff --check; source patch applies with git apply --check |source-patch-check.log; final-diff-check.log |

The sole full-suite failure is parameter_slot_characterization's d7_predicate_and_singular_parameter_rows_are_complete at examples/parameter_slot_characterization.rs:757. Input `const take = later => later;` produces a later binding at bytes13–18 while the fixture expects empty bindings. Base fails with identical actual/expected payloads; the example source matches base exactly. This inherited, out-of-scope failure is neither fixed nor re-baselined. The explicitly ignored slice_elem_variant_reserved integration test is the single full-suite skip (future SliceElem). Rust Tier-A quick is excluded **before execution**: rust-analyzer default build scripts/proc-macro execution conflicts with static-analysis-only authority and can write outside the granted roots. Full multi-corpus Tier-A is human-triggered and was not run. The adapted matrix/quick commands reuse installed read-only Python and the included prism-bound.py cache adapter; they do not claim the literal unmodified uv/no-cache command. The final base/head quick controls each immediately follow a release rebuild in this same worktree; all14 prototype source bytes were snapshotted and restored exactly between them. The earlier frozen-base control is superseded for this gate by the fresh controls.

No strict-clippy green claim is made. Existing test warnings are preserved rather than silently repaired. Product fixture adjustments retain their original ownership/binding controls while replacing or explicitly refusing the formerly required E5 false Exact paths. They are documented in PROBE-LOG P7/P10 and visible in the source patch.

The four timed-out/killed producer roots and private corpus F are unverified. Baseline call producer exclusions are paired on head and included with their actual wall times; they are never counted as zero cost. Whole-public site/callee/consumer measurement is bound to frozen binaries and source snapshots. Sampled call navigation and TypeScript quick are limited samples, not native proof of every remaining target.

Source patch validation uses a fresh read-only git archive extracted into the evidence root; git apply --check writes no Git state. Final docs patch and custody hashes are recorded in PATCHES.md.
