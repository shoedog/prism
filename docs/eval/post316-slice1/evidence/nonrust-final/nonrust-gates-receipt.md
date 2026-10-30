# Slice 1 final non-Rust receipt

**Candidate source:** commit `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7`; tree
`32be14a0eebecd8b936f94c289f21faeb0526716`. All builds used isolated targets under
the original evidence root `/private/tmp/prism-post316-orchestration/final-gates-slice1/nonrust/`;
no worktree `target/` was used. Frozen executables remain at that root and are bound below.

## Results

| Gate | Result | Evidence |
|---|---|---|
| Grammar reproduction | PASS | `node-grammar.log`: both unchanged baselines and shipped TS/TSX parser trees reproduce byte-for-byte. |
| Node/authority active population | PASS | 46 modules, 786 tests: 785 pass, 0 fail, 1 expected grammar-archive skip. Broad rerun: 733 tests, 732 pass, 1 skip; four-module supplement: 53/53. |
| Standalone callable authority | PASS | pinned TypeScript 5.9.3 and profiles; 40 results, `failures: []`. |
| Python deterministic + adoption unit | PASS after real-binary closure | Initial full collection: 937 passed, 3 skips. Exact frozen-binary supplement: 3/3 passed, covering those three named skips without rerunning the collection. |
| Tier-A matrix | PASS | 159/159 emitted rows `ok`, using frozen `prism-matrix`. |
| Tier-A quick | INVALID | Report exists and was read; see below. This is not a regression or accuracy pass. |

## Frozen executables

| Purpose | Frozen path | SHA-256 |
|---|---|---|
| Node native membership census | `frozen/native/project_membership_census` | `23528ffae848a15a90257f7d62a7321cf4b98f016bb3612c5afd8ec658ed846b` |
| Tier-A matrix `prism` | `frozen/matrix/prism` | `947acb87e7f24ad1d64bed834d381794d717753be6fc52bdb91ee0f6d3f69810` |
| Tier-A quick `prism` | `frozen/quick/prism` | `947acb87e7f24ad1d64bed834d381794d717753be6fc52bdb91ee0f6d3f69810` |
| Python real-binary `prism` | `frozen/python/prism` | `8d9b8c5d33a08d4c88e07120fa26a5f545cad749e249bbe3d78de882bf0ddd31` |
| Python real-binary `prism-mcp` | `frozen/python/prism-mcp` | `5acf8529797c6eb00bf0cce41dfc6c48823d83e6b542ed2a2fa6aebb92efaa3d` |

The Node compiler was the restored public TypeScript 5.9.3 module, SHA-256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.

## Retry and attribution accounting

The first Node broad command is retained in `node-invalid.log`: 702 tests, 700 pass, 1 fail,
1 skip. It is setup-INVALID, not a candidate finding: `membership.test.mjs` asserted the
missing `PRISM_MEMBERSHIP_NATIVE` before its test body. The native helper was then freshly
built/frozen and the single diagnosed Node rerun passed. No second Node retry occurred.

No same-environment base control was needed: there was no unexpected behavioral failure to
attribute. Tier-A quick's result is independently invalidated by policy/oracle conditions,
not a candidate regression. A base quick would retain the same pin-policy mismatch question
and would not discriminate a candidate defect.

## Tier-A quick report

The quick command used the independently rebuilt frozen `quick/prism`, `--sut-bin`,
`--allow-stale-sut`, locked offline uv environment, and unique date. Its report records
`baseline_invalid: true` for two separate reasons:

1. policy refusal: `corpus_sha_drift: 07ed5ceb6098 != pinned 20c8490591a3`;
2. oracle incompleteness: `stratum C-method: 4/6 successful probes` (oracle error rate
   `0.06666666666666667`).

It did reach the frozen candidate: SUT error rate is `0.0` and its internal matrix contains
159 `ok` rows, but neither fact establishes clean accuracy while the run is invalid. Its
pinned records are `target-c-method=flip_candidate` (expected `known_fail`),
`module-deps-feature-gated=missing` and `load-repo-feature-gated=missing` (each expected
`oracle_miss_site`), and `ambiguous-symbol-contract=ok`. Do not treat the flip candidate,
missing records, or zero SUT error rate as adjudicated candidate behavior. No rebaseline or
quick retry is authorized.

Preserved external report copies: `tier-a-quick-report/run.json` and `report.json` SHA-256
`68b17467921501c6548f25996a0cdafa2525dab87d1953959a4b47891a17ae38`;
`report.md` SHA-256 `2d0669e79b3f7ff1a25a933406f4c22a751c8c340b544ceb2344ead5caa6cda6`;
snapshot SHA-256 `9903fdbd1afcc18bef7d68c6417ab8cf9764e31de0d9c638ab86e4b79a666a91`.
The uniquely dated harness outputs were copied out then removed; no source report was retained.

## Exclusions and deliberate skips

- `docs/eval/receiver-closure/audit-imported-props-source.test.mjs`: three historical
  source-custody tests excluded. It requires all of `PRISM_AUDIT_TYPESCRIPT`,
  `PRISM_AUDIT_UPSTREAM`, `PRISM_AUDIT_SLICE`, `PRISM_AUDIT_SITES`, and
  `PRISM_AUDIT_SOURCE_REPO`; the historical `real-sites.jsonl` input is absent. No fixture
  was recreated.
- One Node grammar self-test skips its tamper authentication branch unless
  `PRISM_GRAMMAR_ARCHIVES` is supplied. The separate restored-public-input grammar reproduction
  gate passed.
- `eval/adoption/tests/test_prism_adoption.py` remains excluded: live adoption was not
  authorized and `PRISM_RUN_LIVE_EVALS=1` was not set.
- Full Tier-A `--corpus all` remains human-triggered and was not run. No baseline changed.

Initial Python skip names/reasons, now closed by the frozen-pair supplement: 
`test_matrix_against_real_binary_has_no_regressions` (release `prism` absent),
`test_resolve_matched_binaries_against_real_binaries`, and
`test_warm_gate_real_cache_distinguishes_cold_from_warm` (both require on-disk
`PRISM_BIN`/`PRISM_MCP_BIN`).

## Log manifest

| File | SHA-256 |
|---|---|
| `node-invalid.log` (retained invalid attempt) | `983da3f2288b7462b4491badec1ab7361973c86cfcfaf94f5e184507a43b9cfa` |
| `node.log` | `29850fb2290f607a0b8f3746fe20e1bedec9eec03806f96dc58957b8df9836a5` |
| `node-supplement.log` | `de4a0f14cc68ffda50477b0585d684423df8a35794c4d0663c1c616b4d7e362e` |
| `authority.log` | `8746d8f89c3efa5029869de3c0d1d0297f2499da22e20d02b922bc6576a9b57f` |
| `python.log` | `954528c188e1de6c33c0b6100657017cd74fe3ddc2ecc52d06db606e0d777fbe` |
| `python-real-binaries.log` | `4909805a973ed65cbb90d3d1a474278e26322468d0d370aea834328fa4cd95e0` |
| `tier-a-matrix.log` | `411a6f90bb8b6cd6a800532ac3bf4b0bacd4059fcd5fd024fb5ed958ff04a649` |
| `tier-a-quick.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (empty stdout; report above is the result) |
