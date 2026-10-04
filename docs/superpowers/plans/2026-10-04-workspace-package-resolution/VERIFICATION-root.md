# Lane PKG R1 verification

Checkout `/Users/wesleyjinks/code/prism-pkgres`, branch proto/workspace-package-resolution, HEAD 92c1d0bc05ac90f4d2505f2deb2eb309f08e4318 plus dirty R1 source. Evidence root E is `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1`; packet P is `docs/superpowers/plans/2026-10-04-workspace-package-resolution`; pinned TS is `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.

## Verified

Final full-suite command from the workspace, with output full-r1-final.log:

```bash
PRISM_TYPESCRIPT=/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --offline --features mcp
```

**5163 passed / 0 failed / 1 ignored**, 31 result records including **two doctests**, with 24 package regressions. No environment exclusions or out-of-scope failure were silently repaired or re-baselined. The reserved SliceElem test is the sole ignore. The requested single `cargo nextest run --offline --features mcp --test-threads 4` passed 5160/0/1 in 159.737s before the final typesVersions guard; it cannot certify final source.

Permanent-source checks:

```bash
cargo fmt --all --check
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo clippy --offline --all-targets --features mcp
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 scripts/mutgate/mutgate.py --since HEAD --scope fn --lane mutants/lane-pkg-resolution.json --jobs 2 --out /Users/wesleyjinks/prism-evidence/pkgres/repair-r1/mutgate-r1-final
```

fmt/clippy pass. Clippy emits 182 lib-test warnings (137 duplicates), including a boolean-simplification warning in current js_paths.rs. No blanket pre-existing attribution is made. Advisory mutation result: **26/26 selected, admissible and killed**. The registry records repaired intent for the tuple-payload witness, manifest-absent index witness and reachable absence witness. Compile/setup errors are not kills.

The committed-prototype control was restored with read-only git archive and run in the same environment. Ten selected graph-path tests are **0 pass / 10 behavioral failures** on 92c1d0bc (red-control-final.log), GREEN on final R1. Status-only APIs did not exist before R1 and are excluded from behavioral RED attribution. Negative checks now inspect resolved_targets rather than the deprecated null exact_target field. Additional root typesVersions subpath behavior is admitted by early R1 but absent from prototype/main; native other.ts versus early R1 feature.ts identifies the R1 regression. The expanded gate enumerated 144 bad cases before the guard was moved before file probing, then all pass.

Rerun the generated acceptance gate into new output directories (set E/P/TS to the paths above):

```bash
python3 "$P/probes/differential-generate.py" "$E/new-matrix"
python3 "$P/probes/differential-run.py" "$E/bin/r1-resolution" "$TS" "$E/new-matrix/manifest.json" "$E/new-differential"
python3 "$P/probes/cache-invalidation.py" "$E/bin/r1-prism" "$E/new-cache-checks"
```

Final matrix-accepted/manifest.json → differential-r1-final: **5096 cases, 2242 Bound / 861 ProvenUnresolved / 1993 Unsupported**, zero wrong/false-absence/native-expected mismatches. The runner compares module plus owner and complete callable declaration span. Native-bound Unsupported count **1651**, broken out by feature/reason in R1-REPORT and MEASUREMENTS. Package imports are the authorized minimum Unsupported implementation; omitted features cannot justify S2-O9 absence. Persisted caches: **12 checks**, genuine full hits for both artifacts without rewrites, edits invalidate both, complete cold/warm byte parity. PRISM_NAV_EDGE_CACHE_LOAD_DIRTY=1 applies only to these synthetic subprocesses.

Tier-A used an immediate same-worktree `CARGO_INCREMENTAL=0 cargo build --offline --release --bin prism`, then from this checkout's eval directory:

```bash
PYTHONDONTWRITEBYTECODE=1 /Users/wesleyjinks/code/slicing/eval/.venv/bin/python -m tier_a.cli --matrix-only --allow-stale-sut
```

**178 OK / 0 regression / 0 skip**, current eval module/current rebuilt CLI, not the wrong root-cwd editable module. No install was needed. This is a separately rebuilt same-source executable, not a claim that its bytes equal the retained MCP CLI. Custody-rebuild investigation confirms the measured MCP executable matches target/release/deps/prism-3ba4b1893b0b8e1d; top-level Cargo executable differs after mixed feature/transitive dependency builds. The top-level executable was not substituted into measurements. All 547 production/build/script/vendor inputs, 447 test-file hashes and mutant registry remain unchanged across the custody rebuild. Final census timing and both executable hashes are explicit in final-source-binding.json.

S1b-4: **411 unchanged scenarios / 1234 byte-identical files**, complete site keys agree. Public measure.py against main: X 19219, installed-X 19219, R 953, T 61712; every complete call stream identical, no lost base edges. Final public refresh reuses main outputs after input rehashing; a fresh main run was also executed this turn. Module audit checks source populations/hashes and no lost/changed main proof; the sole installed-X addition has a fresh native target/owner certificate. Public STOP was not triggered.

Controller script was not executed against F. `bash -n`, Python probe AST/JSON parsing and Node --check validate packet preparation (packet-checks.json). Its native census/comparison tools are exercised in public measurement. Production, test, helper/probe/oracle and immutable executable hashes are in BUILD-MANIFEST.json. Patches use the two distinct bases and were application-checked outside Git; final-custody.json records the final snapshots and byte reconstruction. No Git writes or off-machine-backup claim.

## Not verified

- Undefined Opus IDs C3, C15, C17-C21, C23, C24, C26. Clarification requested, no public fixture source supplied. No invented repro counts.
- Inherited ordinary lane-P C16b JSX behavior; package TSX now has its own fail-closed JSX-option fence.
- Private F/controller execution; new S2 scratch execution or adoption; independent review; performance/RSS or new cost limits; off-machine backup.
- Full Node/TS conformance, broad discovery/language/config coverage, detached/all-features suites; Unsupported cases remain explicit.
- A final-source nextest repeat: the brief requests one run; final source instead has the complete Cargo MCP suite after repair.
- Tier-A quick (brief explicitly skips it) and human-triggered full multi-corpus runs.

The earlier packet's 58 controls and S2 0/132 result are historical planning evidence, not fresh R1 gates. Neither a green gate nor a snapshot accepts a new refusal cut, generated-output substitution, virtual-loader policy or S2 adoption.
