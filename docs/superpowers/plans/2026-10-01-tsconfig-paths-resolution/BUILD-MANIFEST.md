# Lane-P round-1 source and evidence manifest

MEASURED: planner `plan/tsconfig-paths` HEAD `d53cacbd4ad1503140b544e1e89a59d23fb40aa5`; base `5048f44300a7bb8161444e83c0d02713529a33fd`. READ: controller's previous prototype is `93494168`; it is superseded by this cumulative R1 patch. No Git writes, private F reads, installs or network were performed. The 18 pre-existing eval snapshot deletions are outside this task and preserved.

## Final source custody

MEASURED: **37 owned paths**, exactly `target/paths-proto/P1-owned-files.txt`, agree across current source, archive and patch replay. The replay is `target/paths-proto/replay-r1-membership-final/`; receipt `target/paths-plan/r1/final-evidence/patch-replay.log`. `source-binding.json` records both equalities and **1,444 non-owned tracked build/source/test/eval inputs equal to the bound base**. The private-name exclusion count is zero. The previous 1,445 count included js_exports.rs, now an owned path. No existing fixture expectations were edited outside ownership.

| Owned artifact | SHA-256 |
|---|---|
| target/paths-proto/P1.diff | 33f8fc33a0145952c28c676b5c3b85579cceb31ee39812ea0dbc81b81ecc049c |
| target/paths-proto/P1-owned-files.tar.gz | 4a60a0ee117d230fc101fb47b66e54d44d1539916d2b8ba307eb4836b2cfeceb |
| target/paths-proto/source-hashes.json | b12d47e4cc60ebf6ff568a5bf509e1fed3db326664609f716d243ff15fc7e52d |
| target/paths-proto/P1-owned-files.txt | 37ac50dc456248f6f870bb52d629a0858e9964da3443415307369e4fba31cfa7 |

MEASURED: size **838 added / 20 removed honest production source lines**, **574 added test-code lines**, **1,538 declarative test-data physical lines**, **70 added Tier-A fixture lines**. Integration Rust source is 580 physical lines. Per-file counts: `target/paths-plan/r1/final-evidence/size.json`. Data is reported separately; no numeric LOC cap is asserted.

## Immutable binaries and compiler

MEASURED: release build `CARGO_TARGET_DIR=<workspace>/target cargo build --release --offline` ran in `target/paths-proto/repo`; log `final-evidence/build.log`. The executable was copied once to the immutable head path before measurement. Base reports `slicing 3.1.2 (5048f44300a7)`; head reports `slicing 3.1.2 (d53cacbd4ad1-dirty)`. The latter is inherited build metadata, not implementation-parent/source proof. File hashes, patch replay and suite source hashes bind the modified body. READ: base and fact executables were retained from the earlier base build, originally bound in `target/paths-plan/checkpoint.json`; both were freshly executed against the final public/synthetic inputs in this environment.

| Binary / compiler | SHA-256 |
|---|---|
| /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/base/prism | 8fd563208316411d799e0ef828f09a3ef66f6ca5151031e02fda578642295299 |
| /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/r1/head-membership-final/prism | 40cbbd8f970477fa8e28063025bc6a592c4fd0aa95d1522767cc2487b2c39b95 |
| /Users/wesleyjinks/code/prism-paths-plan/target/paths-plan/base/dump_imports | b00fbeba53b3ca3f7516a3e0f8df74540c39ac84c80c6ebea298db5d7a4f2644 |
| /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js | 3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675 |

MEASURED: current `oracle.cjs` SHA-256 `c95bb36ff2df5db38e0e6db9ec80b8b351727f569f7242c969621fbcb81a55fa`; `CONTROLLER-paths.sh` SHA-256 `3c97b3afc3aca92765504801c3b77a9779f112591c78321fb6d303813a4ba2d7`. Full probe hashes and receipt hashes are in `final-evidence/BUILD-RECEIPTS.json`. TS compiler version/bytes are checked in the aggregate script. Recoverability is effective explicit Node/Node10 and respects jsconfig/delegated barriers; TS2308 checks the TSX first-wins star conflict; JSX without checkJs retains that symbol-lookup blind spot, while production refusal is verified in both grammars.

## Fresh execution receipts

MEASURED: all integration mutations completed before the final three suites, serially with one shared Cargo target. Every suite's `*-totals.json` contains identical before/after hashes for all 37 owned files, matching the frozen source and final archive. No concurrent mutation or live binary overwrite is admitted. Final `verification-summary.json` records source hashes after formatter/clippy too.

| Final execution | Passed / failed / ignored | Groups | Log SHA-256 |
|---|---|---|---|
| default | 4770 / 0 / 1 | 29 | 7ea7a9848568a56ac3151263365a61ba8575f11b51204dadbc347935fbd434ea |
| mcp | 4963 / 0 / 1 | 31 | cc7256b3003945c6ce720eaa10161b1fb3bc3203ba95930e45bbb8ebbe8046e5 |
| all-features | 4986 / 0 / 1 | 31 | af7b5fe1e0c9e9dc22a7b065f58c4220a4197db41f60ded4c04b0c1a74a42a7a |

MEASURED: matrix **169 passed / 0 failed**, immediately preceded by the release rebuild in the same worktree; log SHA-256 `019485d7afec61c2e017c985cc4d469c225738c069d280a99f84fb0a3a3706a9`. Installed Python 3.12 ran `-B -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin <immutable head>` from prototype/eval. No install or denied uv retry. Formatter passes; clippy passes with **139 library warnings**. All-features uses PRISM_TYPESCRIPT bound to the offline package. Existing ignored SliceElem test remains ignored; no unrelated failure was rebaselined.

MEASURED: controls **152 scenarios / 166 sites / 55 changes** (**53 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL**), zero complete-row preservation/key/metadata violations. Kernel mutants **20/20 killed**; actual-source integration mutants **10/10 killed**, with one selected test and assertion panic per mutant. Logs, copied mutated source and summaries are retained. Cache probe passes cross-binary rejection, cold/full-hit/config/extends/candidate/package parity and unrelated .txt hit preservation. MEASUREMENTS contains full control/mutant tables and exclusions.

READ retained execution, not fresh suite claims: base default 4,750 passed / 0 failed / 1 ignored, 29 groups; base standalone P1 harness 8 behavioral RED / 9 preservation passes, 17 groups. Old prototype binary `255b1185b03c0b6805609219b567876ffdff42b2b7a46e1f9b78b373c59509e5` supplies fresh pre-change synthetic rows; 54 scenarios differ from corrected head. Additional final membership probes retain TS root membership plus concrete wrong outputs. Inadmissible setup/custody outputs are logged separately, not counted as RED/kills/final runs.

## Fresh public expectations and classes

MEASURED: final public X/R/T signature **3,121 / 0 / 0**, all 3,121 X changes CORRECT_STATIC_BINDING; zero other changed classes, added/removed site keys or metadata changes. Count delta from the earlier signature is **0 / 0 / 0**. `final-evidence/public/FINAL-SUMMARY.json` binds each per-row result; TypeScript-derived expected files are fresh and checked against head. Source/context hashes are current and TypeScript-recorded config bytes agree. Corpora are trusted read-only inputs; concurrent snapshot-security guarantees are not claimed.

| Corpus | Expected SHA-256 | Input files |
|---|---|---|
| X | f3b9915cbd9801208b839e5a4593660a756526336656703b42ada266fd93a383 | 640 |
| R | 37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570 | 51 |
| T | 37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570 | 722 |

MEASURED: the public retained-candidate histogram has eight NONRELATIVE_EXPORT_HOP and two UNCLASSIFIED_P1_PROOF in X; R/T none. The name-directed reason classifier avoids attributing unrelated barrel exports to the requested member. Unclassified reasons remain open explanations. Earlier public/verified/accepted/intermediate runs are superseded by **final-evidence/**, except specifically labeled retained or pre-change evidence.

## Controller boundary

MEASURED: CONTROLLER-paths.sh defaults to the immutable head above, passes bash syntax and public-only aggregate execution with 152 scenarios; final wrapper `controller-public-receipt.json` explicitly marks `actual_F_run: false`. Stdout is one JSON object, controller stderr empty; no raw path/function/site identities are exported. Reasons are independent ordered P1 cuts, not production telemetry. READ: prior private F 2,345/3,102 (75.60%) is controller-supplied on older binary/oracle; fresh F counts and histogram must be supplied by the controller. Never open its source or raw evidence here.

READ: prototype-own Tier-A quick is excluded because ignored scratch has no tracked Git source universe and the corpus pin differs; the controller's updated actual worktree must run it. Full multi-corpus Tier-A is human-triggered. Fresh F, independent Opus round 2, owner confirmation, controller application/commits, push/merge and P2 are not performed. Large-tree overhead and snapshot concurrency/security remain unmeasured. No buildable prototype work remains pending locally.

## Local snapshots and controller commits

MEASURED: source/archive/patch replay are complete. Final local destinations are `target/paths-proto/P1-evidence-r1-final.tar.gz`, `P1-plan-r1-final.tar.gz` and `final-snapshot-r1-hashes.json`. The post-write JSON binds snapshot hashes; the manifest avoids self-referential archive hashes. Evidence includes final public raw rows/expected data/logs, source/patch hashes and bound binaries. Intermediate build directories and compiled mutant binaries are excluded; mutated owned source and behavioral logs are included. Local snapshots do not replace controller Git/external custody.

ASSUMPTION controller commit messages: plan `docs(paths): fold Opus spec review round one`; prototype `fix(paths): enforce root membership and skipped-star alias barriers`. Plan ownership: SPEC, MEASUREMENTS, IMPLEMENTOR, OQ-paths, BUILD-MANIFEST, HANDOFF, REVIEWER and changed probes (CONTROLLER-paths.sh, cache_probe.py, controls_gen.py, integration_mutants.py, mutants.py, oracle.cjs), plus new final_receipts.py, changed verify_controls.py and root VERIFICATION.md (16 plan/report paths total). Prototype ownership is exactly 37 paths in P1-owned-files.txt. Exclude the pre-existing eval snapshot deletions from commits. The planner performs no Git writes.

MEASURED verification-gate supplement: `VERIFICATION.md` is at the planner repository root. `hook-suite-revalidation.json` confirms the recorded full-suite output hashes, reparsed totals and unchanged production/test source. New control assertions are GREEN on the final oracle and RED on the d53cacbd oracle; O01 histogram mutant is killed. Final snapshots are refreshed for 16 plan/report paths; P1.diff and the 37 prototype paths are unchanged. Full-suite execution is not claimed to have been repeated for documentation/evidence-assertion-only changes.
