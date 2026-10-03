# Lane-P P1 adopted performance fix — verification stopped

[MEASURED] Checkout `/Users/wesleyjinks/code/prism-paths-impl`, branch `feat/tsconfig-paths-p1`, HEAD `755f85fe953118589aefb9089419b0144f0622f9`, clean at entry. This verification pass stopped on the requested lint gate. No production or mutant-driver changes, Git writes or F access occurred. One verification pass was declared; the only additional execution was the same-environment attribution control for the failed lint gate.

## Failure

**SMELL 1:** new `clippy::type_complexity` warning at `src/js_paths_snapshot.rs:47`, on `closure_memo: Arc<Mutex<BTreeMap<(String, Option<Vec<String>>), bool>>>`.

**SMELL 2:** new `clippy::type_complexity` warning at `src/js_paths_snapshot.rs:613`, on the boxed reference iterator type.

These are lint concerns, with no demonstrated incorrect behavioral result. They fail the explicit requirement for no new warnings in touched files. Both clippy commands returned status 0; that does not satisfy the warning gate.

Control: r4 `b5e9ed72` was extracted with `git archive` under `target/verify-fable/base-control` and checked in the same environment with the same offline all-targets/all-features clippy command and profile settings. Current touched-file diagnostics: **24**; r4: **22**; source-bound fingerprints identify exactly the two new diagnostics above. Snapshot type-complexity diagnostics: **2 current / 0 r4**. Library warning summaries: **141 current / 139 r4**; library-test summaries: **184 / 182** (include duplicates). The same-environment control rules out a toolchain-only explanation.

Receipts: `target/verify-fable/logs/clippy.log`, `logs/clippy-r4-control.log`, `clippy-summary.json`, `hypothesis-probe-result.log`. An initial regex summary misassociated a generated-warning line with an AST location; that parser output was inadmissible and was replaced with warning-block and source-line matching before this report.

## Every requested check

| Check | Current result / total |
|---|---|
| `cargo fmt --check` | PASS, status 0; empty diagnostic log |
| clippy all-targets/all-features | FAIL warning gate: 2 new touched-file warnings, 0 on r4 at those constructs |
| Default suite | INTERRUPTED, status 130; 1,274 successful test lines, 0 failed lines, 0 completed result groups; no complete suite total |
| MCP suite | Not run; no total |
| All-features suite with pinned `PRISM_TYPESCRIPT` | Not run; no total |
| Tier-A matrix-only | Not run; no total |
| Kernel mutants | 0 run; no kills certified |
| Integration/library mutants | 0 run; no kills certified |
| Resource mutants | 0 run; no kills certified |
| Mutant staleness | Not assessed; no driver changed |
| Packet cache probes, including node_modules edit/add/remove | Not run; no states certified |
| Packet controls versus r4 | Not run; no rows certified identical |
| S1b-4's 411 controls | 0 run; byte identity unverified |
| Plain X versus base and oracle | Not run; expected 3,121 not certified |
| Installed X versus base and oracle | Not run; expected 3,121 not certified |
| R versus base and oracle | Not run; expected 0 not certified |
| T versus base and oracle | Not run; expected 0 not certified |
| Installed X performance | 0 of 3 alternating repeats; no wall/RSS medians or ratios |
| Nx exact performance | 0 of 3 alternating repeats; no wall/RSS medians or ratios |
| Nx bundler performance | 0 of 3 alternating repeats; no wall/RSS medians or ratios |
| Nx wildcard performance | 0 of 3 alternating repeats; no wall/RSS medians or ratios |
| Controller F script pointer | Not changed; still historical r4 binary |

Performance gates remain **wall <=1.30x**, **RSS <=1.20x**. No performance child ran, so there is no current host-load attestation or measured gate number. Fable's supplied **1.179x wall / 1.135x RSS** installed-X result is inherited from its REPORT.md and was not reverified here.

The default suite was launched before the lint comparison result was inspected. It was interrupted with Ctrl-C after the gate failure was recognized. Its 1,274 successful lines are partial execution evidence, not a green suite or a full test total. No failed test output was observed; the uncompleted tests remain unknown. `target/verify-fable/default-interrupted.json` and `logs/default.log` retain this distinction.

## Binding, custody and exclusions

Production/tests/vendor/build inputs are SHA-256 bound in `target/verify-fable/source-binding.json`. Rust and clippy are 1.94.0. Commands used `CARGO_TARGET_DIR=target/verify-fable/build`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, and offline Cargo; checks ran serially. No release binary was built or certified.

Pinned TypeScript path: `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`; SHA-256 **3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675**, checked this turn. That path was supplied to the interrupted default suite and is now recorded explicitly in BUILD-MANIFEST.md.

All later checks were excluded because the user required stopping on any failed gate. F was never opened. No fresh corpus oracle, cache validation, full suite, mutant census, performance measurement, release binding or controller-pointer update is claimed. A process-census attempt using `ps` was sandbox-denied and provided no process-state evidence. All tool-managed verification commands returned before cleanup.

Disposable build and r4-control directories were removed at the end; lean logs, summaries and local snapshots remain in `target/verify-fable/`. Existing earlier-round evidence/build directories were retained. `custody-final.json` and `stop-snapshot.tar.gz` bind the final docs and receipts locally; no external backup or Git custody is claimed. See [VERIFY-FABLE-FILES.md](VERIFY-FABLE-FILES.md) for files and the proposed controller commit message.
