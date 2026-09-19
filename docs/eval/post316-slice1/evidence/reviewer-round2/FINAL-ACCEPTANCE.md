# Slice 1 final independent acceptance

APPROVE — **WRONG: 0 / SMELL: 1**. Required verification is reported with the explicit limits below. This is final evidence reconciliation after two source-review rounds and one authorized, converging test-only cap closure; it is not another review round.

## Accepted artifact and findings

Approved source/test commit: `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7`, tree `32be14a0eebecd8b936f94c289f21faeb0526716`, base `f5350044a18bf95f3deb51a41b4b09e9d585b134`. All ten live source-manifest hashes were independently checked against that commit. Subsequent receipt/handoff/evidence changes do not change this source approval.

Round-one class-baseline WRONG is fixed and all finite test/evidence gaps are closed. The remaining nonblocking SMELL is repeated full function-inventory reconstruction per captured call; no measured performance regression is claimed. See `REVIEW-round2.md` for mechanisms, exact test patch identity, complete endpoint checks, cache evidence and review-cap disposition.

## Verification totals

Broad suite executions are **supplied by implementor/verifier**, with raw logs, summaries and receipt hashes independently inspected here; they were not broadly rerun by the reviewer. Earlier focused/base controls and cache probes in the round-two report were independently executed.

| Gate | Verified result |
|---|---|
| Rust default | 29 suites: **4,518 passed / 0 failed / 1 ignored** |
| Rust MCP | 31 suites: **4,711 / 0 / 1** |
| Rust MCP + detached-owner-audit | 31 suites: **4,734 / 0 / 1** after one diagnosed compiler/profile environment retry |
| Widest examples | 4 suites: **32 / 0 / 0** |
| Clippy | completed successfully; **179 warnings, 134 duplicates**; no assertion that warnings are new or absent |
| Formatting / diff whitespace | clean |
| Node active population | **786 selected: 785 passed / 0 failed / 1 expected skip**, across supplied 46-module inventory (732+53 passes across broad rerun and supplement) |
| Standalone callable authority | JSON contains **40 results, failures=[]**, pinned TypeScript 5.9.3 |
| Python deterministic/adoption-unit coverage | **940 covered passes**: 937 initial passes + exact frozen-binary supplement closing three initial skips |
| Tier-A matrix | **159/159 `ok`** rows |
| Tier-A quick | **INVALID**, report present and inspected; not an accuracy pass or attributed regression |

Independently verified frozen native/matrix/quick/Python executable hashes, TypeScript compiler hash, non-Rust log hashes, Rust gate log hashes, final source pins and all four quick-report artifact hashes. Non-Rust receipt hash: `be07e1fe60e275181efecedb8d46373adea554be11e381e3cb42c631e64b028a`.

## Exact exclusions and invalid observations

1. **Tier-A quick remains INVALID** for `corpus_sha_drift: 07ed5ceb6098 != pinned 20c8490591a3` and `stratum C-method: 4/6 successful probes`. Its oracle error rate is 0.06666666666666667; SUT error rate is 0.0. The zero SUT error rate and its internal 159 successful matrix rows do not validate accuracy. Pinned observations are `target-c-method=flip_candidate` (expected known_fail), `module-deps-feature-gated=missing` and `load-repo-feature-gated=missing` (expected oracle_miss_site), and `ambiguous-symbol-contract=ok`. They remain unadjudicated invalid-run observations; no rebaseline or retry occurred.
2. Three historical source-custody tests in `docs/eval/receiver-closure/audit-imported-props-source.test.mjs` were excluded because the required historical `real-sites.jsonl` input is absent. No replacement fixture was invented.
3. One Node grammar tamper-authentication branch was skipped because `PRISM_GRAMMAR_ARCHIVES` was not supplied for that self-test. Separate reproduction of both unchanged baselines and shipped TS/TSX parser trees passed byte-for-byte.
4. `eval/adoption/tests/test_prism_adoption.py` was excluded. Its source explicitly requires `PRISM_RUN_LIVE_EVALS=1`; live adoption was not authorized. Adoption unit tests are included in the 940 covered passes. Full Tier-A `--corpus all` was not run because it is human-triggered.
5. First widest Rust run had 23 compiler/profile `NotPresent` setup refusals; one diagnosed environment retry passed all 23. First Node broad run had missing `PRISM_MEMBERSHIP_NATIVE` setup refusal; one diagnosed rerun with the freshly frozen helper passed. Both original logs remain retained and are inadmissible as candidate behavioral failure evidence. The three initial Python real-binary skips were individually closed by the frozen-pair supplement, so they are not remaining exclusions.

No source-change failure is attributed from INVALID/setup observations. There was no remaining admitted broad-suite behavioral failure requiring a new base attribution control. The original sixteen candidate-only failures were already independently controlled on unchanged base and reconciled during round one.

## Acceptance scope

The bounded ownership repair and its preservation/refusal/cache behavior meet acceptance with the disclosed verification limits. This approval does not establish class execution correctness, broader callable support, public accuracy improvement, live adoption, publication, CI, merge or production effects. Source ownership, resolved callee evidence, DFG evidence and public slicing observations remain distinct.

Controller may finish durable evidence/handoff custody and route the next authorized sequential slice using this accepted source predecessor. No further source changes or test expansion are requested by review.

Evidence receipts: `/private/tmp/prism-post316-orchestration/final-gates-slice1/rust-gates-receipt.md` and `nonrust/nonrust-gates-receipt.md`; lane copies are being folded by the implementor.
