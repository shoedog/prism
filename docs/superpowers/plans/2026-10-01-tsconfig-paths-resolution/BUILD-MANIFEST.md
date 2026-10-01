# Build and evidence manifest — lane P

MEASURED: resumed planner checkout `plan/tsconfig-paths` at `5e7570c3aad19b3aa08e832a92d153d4691887fe`; implementation/base parent `5048f44300a7bb8161444e83c0d02713529a33fd`. Controller commits `33e6391d` and `5e7570c3` hold the earlier packet and interim OQ dispositions. No Git write was run by this planner. This manifest binds the modified prototype by file/patch/binary hashes, rather than its inherited Git identity.

## Final prototype custody

MEASURED: 35 owned paths agree across final source, owned-file archive and exact patch replay. An initial 259-path production/build-input check passed; an expanded check then verified all 1445 non-owned tracked files under src/tests/scripts/eval and Cargo/build inputs against the base, with zero private-name paths encountered (`resume-all-input-hashes.json`). This also verifies that existing test/fixture expectations were not changed. See `target/paths-plan/resume-binding.json` and `patch-replay-final.log`. `target/paths-proto/replay-final-v2/` is the verified replay. The earlier patch/archive is superseded by the final position-injection test compile fix.

| Artifact | SHA-256 |
|---|---|
| `target/paths-proto/P1.diff` | `ef6f07891471fb287d6b5ff5d35cca85823c6fe15a7682116bfd6615287c089f` |
| `target/paths-proto/P1-owned-files.tar.gz` | `38449873e57b284db5b098e83a55f2c35d0c6b470ee5978807d9db065730de1a` |
| `target/paths-proto/source-hashes.json` | `bb696c829e4f1e46db364f7d879538883a692e78c45dba6b0ed48c1085b78fc4` |
| `target/paths-proto/P1-owned-files.txt` | `a2231602ef421f2e15f54689e5151d8e4d985bec0a9d1b57012638f41ac91b5a` |

MEASURED: final size is 671 added / 18 removed honest source lines, 506 added test lines and 70 added fixture lines. The integration test is 511 physical lines. Full per-path accounting is `target/paths-plan/size.json`. READ: the explicit owned-file list and P1 dispatch are in IMPLEMENTOR.md.

## Binaries and oracle

MEASURED: an immediate `CARGO_TARGET_DIR=<workspace>/target cargo build --release --offline` in `target/paths-proto/repo` completed; build output is `target/paths-plan/resume-build.log`. The rebuilt executable was copied to the final head path. The current dirty planner Git identity is build metadata, not the implementation parent.

| Artifact | SHA-256 |
|---|---|
| `/Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/base/prism` | `8fd563208316411d799e0ef828f09a3ef66f6ca5151031e02fda578642295299` |
| `/Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/head/prism` | `255b1185b03c0b6805609219b567876ffdff42b2b7a46e1f9b78b373c59509e5` |
| `/Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/base/dump_imports` | `b00fbeba53b3ca3f7516a3e0f8df74540c39ac84c80c6ebea298db5d7a4f2644` |
| `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` | `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` |

MEASURED: base reports `slicing 3.1.2 (5048f44300a7)`; rebuilt head reports `slicing 3.1.2 (5e7570c3aad1-dirty)`. READ: the retained base/fact executables were built before prototype changes in the earlier uninterrupted planner segment; their original base binding is in `target/paths-plan/checkpoint.json`. The fresh public rerun uses those same base executables as the same-environment control.

## Execution receipts

READ: the completed full-suite executions were retained across the interruption and were **not rerun** during receipt completion. MEASURED: this resume reparsed every result group and matched the saved totals (`target/paths-plan/resume-suite-receipts.json`).

| Retained execution | Passed | Failed | Ignored | Groups | Log SHA-256 |
|---|---:|---:|---:|---:|---|
| `base/cargo-test.log` | 4750 | 0 | 1 | 29 | `8678f2e85e90b3dffb163fa966e473782f6bf2b408b66aaa8298caee00bb9e94` |
| `proto-full-test.log` | 4960 | 0 | 1 | 31 | `c29239f07d00c4d13fc449a33292d5738b7302be452448936ed3a57c028d7724` |
| `proto-all-features-test.log` | 4983 | 0 | 1 | 31 | `ba76f3e1ef14a453d6d7dc3d59672f96527ff2960c6c7fab9ad9aee533e16ccc` |

MEASURED: the immediately rebuilt final prototype Tier-A matrix was rerun: 169 fixtures passed, zero regressions. Receipt: `target/paths-plan/resume-tier-a-matrix.log`, SHA-256 `019485d7afec61c2e017c985cc4d469c225738c069d280a99f84fb0a3a3706a9`; parsed result is `resume-matrix-receipts.json`. The command used the existing Python 3.12 interpreter with `-B -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin <rebuilt binary>`, from the prototype eval directory; no install or uv cache retry.

READ retained controls: `controls-verification.log` records 76 scenarios / 84 sites / 30 changed rows / zero preservation violations. Independent classes are 28 CORRECT_STATIC_BINDING and 2 CORRECT_STATIC_REFUSAL. `mutants-summary.json` records ten kernel kills; `integration-mutants-summary.json` records four integration kills, with behavioral assertion output retained in their directories. `cache-probe.log` records cache/caller controls. `proto-fmt.log` is empty (formatter passed); `proto-clippy.log` records 139 library warnings and successful completion. No warning-free or newly executed mutant/cache/suite claim is made.

## Final public rerun

MEASURED: the final same-environment base/head/TypeScript rerun completed. X has 19,219 sites and 3,121 changed rows, all CORRECT_STATIC_BINDING; R has 953 sites and 0 changes; T has 61,712 sites and 0 changes. Every new edge changes empty UnknownName to exactly one Exact/import_member and matches the independent TypeScript import/module/callable/name/span proof. Added/removed keys and non-resolution metadata changes are 0 in every corpus. No other class is present. Fresh rows and import facts reproduce the earlier base/head rows exactly.

MEASURED: authoritative receipt `target/paths-plan/resume-public/FINAL-SUMMARY.json`; raw X/R/T dumps, rowdiff, P0 and per-row TypeScript classifications are in the same subtree. `source-input-hashes.json` binds 640 X, 53 R and 721 T source/context files; TypeScript-recorded in-root config/context hashes were checked against current bytes. Corpora are READ trusted read-only inputs; this is not a concurrent snapshot-security claim. `target/paths-plan/resume-public.log` records command completion. MEASUREMENTS.md contains the complete P0 mechanism table and final changed-row classes.

## Controller-only F and exclusions

MEASURED: `probes/CONTROLLER-paths.sh` passes `bash -n` and an actual execution using only the public C01 TSX synthetic fixture. One aggregate JSON object, one correct binding, no stderr or raw identities: `target/paths-plan/controller-public-smoke/receipt.json`. This smoke is not F evidence. Script SHA-256: `9bb2ba42d1c5f6c3c3584410cdcc3d3f3bf1e78a445ea140c1f5e3b4022e2e73`.

READ: actual private F, the prototype own-worktree Tier-A quick, full multi-corpus Tier-A, external Opus review, owner confirmation, Git commits/worktree creation, push and merge were not performed. The prototype quick has no admissible receipt because ignored scratch source has no tracked universe and its committed pin differs; see `tier-a-quick-inadmissible.json`. Full multi-corpus is human-triggered. Snapshot concurrency/security and performance overhead remain unmeasured.

READ: inadmissible setup probes and the source/archive correction are recorded in `target/paths-plan/PROBE-LOG.md` and `RESUME-PROBE-LOG.md`. No setup failure is counted as semantic RED or mutant kill.

## Final snapshots

READ: final local archive destinations are `target/paths-proto/P1-evidence-final.tar.gz` and `target/paths-proto/P1-plan-final.tar.gz`; `target/paths-proto/final-snapshot-hashes.json` is the post-write hash/verification receipt. These archives are created after this packet is reconciled, avoiding a self-referential archive hash in its contents. The evidence archive includes source/patch/hash artifacts, base/head/fact executables and public raw/text evidence; large intermediate test libraries/executables and compiler/build directories are excluded. The packet archive contains this final planning packet. Local archives are snapshots; controller commit and external custody remain open.

ASSUMPTION controller commit messages: plan `docs(paths): finalize lane-P receipts and P1 dispatch`; prototype `feat(paths): resolve finite tsconfig paths import members`. Plan owned changes are SPEC.md, MEASUREMENTS.md, IMPLEMENTOR.md, HANDOFF.md, new BUILD-MANIFEST.md and probes/CONTROLLER-paths.sh. Prototype ownership is exactly P1-owned-files.txt (35 paths).
