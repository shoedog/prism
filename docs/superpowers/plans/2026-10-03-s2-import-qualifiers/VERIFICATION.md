# S2 R2 verification — static-binding candidate

## Verified

All observations below were run in this turn on selected source c35719e1 + F1/F2/F5/F6/corrected F8, under owner S2-O7. Exact is static binding; F3/F4/F7 runtime safety is outside this certificate. Actual transcript turn_context confirms **gpt-6.1-sol**, session `01a1067f-91c8-7302-b253-3443cc557ef0`; model-attestation.json records the transcript/cwd. Evidence root: `/Users/wesleyjinks/prism-evidence/s2/repair-r2`.

| Gate | Observation / receipt |
|---|---|
| Full `cargo nextest run --offline --features mcp` | **5,168 passed /0 failed /1 existing skipped**, 157.031s; `nextest-serial.log`, `serial-gate-commands.json`. All 31 S2 tests included. Skip: resolution_test::slice_elem_variant_reserved (existing #[ignore] in source). |
| MCP doctests | **2 passed**; `cargo test --offline --features mcp --doc`, `doctests.log`. |
| fmt | PASS; `cargo fmt --all -- --check`, `fmt.log`. |
| Clippy | PASS; `cargo clippy --offline --features mcp --all-targets --message-format=json`, `clippy.jsonl/.stderr`; 371 warning emissions, no fresh warning attribution. |
| Advisory mutgate | **22 registered /16 selected /16 admissible /15 killed**; S2-02-position-proof SURVIVED; six scoped omissions. All eight selected repair mutants S2-15/16/17/18/26/27/29/30 killed. `CARGO_NET_OFFLINE=true python3 scripts/mutgate/mutgate.py --lane mutants/lane-s2-import-qualifiers.json --since 4e592daa --scope file --jobs 1 --out .../advisory-mutgate`; summary/log. |
| Tier-A matrix | **182 OK /0 regressions /0 skips**; immediate `cargo build --offline --release`, then installed Python 3.12 `-m tier_a.cli --matrix-only --allow-stale-sut --sut-bin ../target/release/prism` from eval; matrix-rebuild.log/matrix.log. No baseline edits. |
| S1b-4 parity | **411 controls /639 sites /822 byte-identical comparisons**, zero differences/stderr; `probes/s1b-parity.py`, s1b.json/log. |
| Combined public yield | **132 /132 /0 /0** for X /installed-X /R /T; delta **0 /0 /0 /0** versus c357 reference. Fresh main/head/facts/native comparison, `probes/compare-head.py ... public --jobs 4`; public/summary.json and per-corpus comparison/binding/receipt/oracle-inputs. Every changed row CORRECT_STATIC_BINDING with native module/owner/full-span agreement. |
| Lane-P preservation | Every populated main row byte-identical; strict unique key populations: X 19,219 sites/10,776 bound; installed-X same; R 953/216; T 61,712/27,452. 49,220 bound-row comparisons across four streams; X snapshots are not additive. R/T complete streams byte-identical; lane-p.json. |
| Regression controls | Five new-cut tests F1/F2/F5/F6/F8 fail on Git-exact c357 source in the same environment; legacy/star opacity and original static alias controls pass (2 pass/5 expected fail of7). c357-red.log and c357-control-binding.json. Omitted only the absent new F8 schema assertion in RED; behavioral assertions unchanged. All31 R2 tests GREEN, targeted-fixed.log and full suite. |
| Patch/source custody | R2-src.patch seven src/tests files relative c357; dry-run/actual application exact, source-patch-check.json. Registry + planning packet in R2-docs.patch with declared parents; application receipt. 770 source/test inputs match final-source-inputs; 762 inputs outside eight repair-owned product paths unchanged, scope-check.json. |

The first targeted assembly run was 30 pass/1 fail: corrected F8 retention guard was omitted. The bounded fix restored it; second run 31 pass. No expectation changed. The first full run reported 5,167 pass/1 launch failure/1 skip: nextest double-spawn ENOENT for lang_c test_threed_slice_c, before any test body ran. That observation is inadmissible for behavioral attribution. Same-environment c357 control passed (1/1); serial full rerun above passed. Overlapping Cargo builds are a plausible launch-failure cause, not proven causation; no product/C test change. Probe-log.md records predictions, alternatives and results.

Head tools were rebuilt from source with the exact offline dependency lock using release build and `probes/build-facts.py --head`; copies in candidate-bin and canonical target/s2-plan/bin. BUILD-MANIFEST hashes pin CONTROLLER-s2.sh's head-prism/head-dump_imports. Main tools unchanged. Cache epochs **CPG118 /navigation74**. No source/test change after the final full suite. Mutgate --since 4e592daa is an ancestor of both c357 and planning HEAD (scope-check.json).

The original static whitelist was already present in c357: its alias/write tests are preservation controls and do not fail that baseline. Corrected F8 legacy opacity also matches c357 by design; S2-30 kills removal of that preservation guard. This distinguishes new-cut RED evidence from inherited behavior.

## Not verified

- **Tier-A quick skipped/unverified**, explicitly directed by repair-r2-brief after the prior >one-hour hang; full multi-corpus Tier-A not run.
- Private F never opened; private head correctness/adoption remain controller-only.
- Authoritative all22 registry mutation run not performed; advisory S2-02 survives and six anchors are omitted. No equivalence claim.
- Runtime safety for F3 dynamic/require/default re-acquisition, F4 namespace enumeration and F7 eval/Function/reflected codegen/host channels is **out of model, disclosed by S2-O7**, not verified. Both-grammar *_out_of_model_* tests pin selected current Exact behavior. Prior reflected-codegen same-C/result1 witness remains valid under the superseded runtime contract; it is not a blocker under the owner-selected static contract.
- Fresh Clippy warning attribution, full legacy E5/native-control replay, all-feature/platform/performance and grammar fuzzing not run.
- Independent controller R2 review, adoption, Git commit/push/publication/merge not performed. No Git writes or network installs.

Supplemental retained-base replay public-reuse preserves the initial measurement files; its final disposition is recorded in supplemental-measurement.json. The primary public/ replay is complete and does not depend on it. uv discovery failed on protected cache access; that probe supplies no Tier-A evidence. Installed Python3.12 ran the in-tree matrix directly.
