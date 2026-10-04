# Lane PKG verification

Base: `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, branch
`plan/workspace-package-resolution`, uncommitted bounded prototype. Evidence is
under `/Users/wesleyjinks/prism-evidence/pkgres/planning`.

The final full suite ran during this task and passed **5,147 tests, zero failures,
one ignored test**, including two doctests (31 result records). The hook follow-up
rechecked every production and test hash against that run before writing this
root report. No production or test code changed afterward; this follow-up changes
documentation and custody only. No environment restriction prevented the full
MCP suite from running.

## Verified

The exact final full-suite command, run from this repository:

```bash
PRISM_TYPESCRIPT=/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --offline --features mcp > /Users/wesleyjinks/prism-evidence/pkgres/planning/full-v2.log 2>&1
```

Results: **5,147 passed / 0 failed / 1 ignored**. The ignored test is
`resolution_test::slice_elem_variant_reserved`, reserved by the existing spec.
Receipts: `full-v2.log`, `full-v2-totals.json`, `final-test-binding.json`, and
`hook-regression-audit.json`. No out-of-scope suite failure was observed; nothing
was re-baselined or silently repaired.

Additional executed commands and their results:

```bash
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --offline --features mcp --test integration js_packages_test > /Users/wesleyjinks/prism-evidence/pkgres/planning/targeted-4.log 2>&1
cargo fmt --all --check > /Users/wesleyjinks/prism-evidence/pkgres/planning/fmt-v2-valid.log 2>&1
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo clippy --offline --all-targets --features mcp > /Users/wesleyjinks/prism-evidence/pkgres/planning/clippy-v2-valid.log 2>&1
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 scripts/mutgate/mutgate.py --since HEAD --scope fn --lane mutants/lane-pkg-resolution.json --jobs 2 --out /Users/wesleyjinks/prism-evidence/pkgres/planning/mutgate-v2 > /Users/wesleyjinks/prism-evidence/pkgres/planning/mutgate-v2.log 2>&1
```

Results: package tests **8/8 pass**; fmt **PASS**; clippy **PASS**, with 181
lib-test warnings, equal to the same-environment base count; advisory scoped
mutants **2 selected / 2 admissible / 2 killed** out of eight registry entries.

The requested single nextest run used
`cargo nextest run --offline --features mcp --test-threads 4` and passed
**5,145 tests / 0 failures / 1 skipped** (`nextest.log`, 317.959s). It preceded the
final native condition/mode corrections and does not certify final source. The
full Cargo suite above certifies the repaired source and adds the two doctests.

Native controls were generated and executed with these commands:

```bash
python3 docs/superpowers/plans/2026-10-04-workspace-package-resolution/probes/controls.py /Users/wesleyjinks/prism-evidence/pkgres/planning/pkg-controls-complete
python3 docs/superpowers/plans/2026-10-04-workspace-package-resolution/probes/measure.py /Users/wesleyjinks/prism-evidence/pkgres/planning/bin/base-prism /Users/wesleyjinks/prism-evidence/pkgres/planning/bin/head-v2-prism /Users/wesleyjinks/prism-evidence/pkgres/planning/bin/head-v2-facts /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js /Users/wesleyjinks/prism-evidence/pkgres/planning/controls-complete --controls /Users/wesleyjinks/prism-evidence/pkgres/planning/pkg-controls-complete/manifest.json > /Users/wesleyjinks/prism-evidence/pkgres/planning/controls-complete.log 2>&1
```

Results: **58 controls pass**, with **42 native-correct Exact additions** and
**16 unchanged/refused edge controls**; zero lost edges or unproven changes.
Each positive control has an empty baseline target list. Using the pre-change
binary as the candidate would fail the runner's required admission assertion.
The hook audit rechecked this for all 42 positives.

Regression and edge coverage:

| Behavior | Pre-change failure witness | Negative or edge witness |
|---|---|---|
| Node10 legacy entries and index fallback | Seven same-environment base package tests fail behaviorally; native index/types/typings/main controls have missing baseline targets | Missing types falls to index rather than main; declaration winner; module field ignored |
| Source-ordered modern exports | Native types-first/default-first controls; `pkg_exports_mode_and_source_order` is RED on base | Missing matching condition falls through; existing declaration blocks fallback |
| Writer import/require mode | Native require/NodeNext/ESM/CommonJS/preserve controls have missing baseline targets; same-environment pre-repair CommonJS test emits wrong `other.ts` instead of `index.ts` | Unsupported/default bundler emit modes decline |
| Exact subpaths and pattern priority | Pattern controls and `pkg_import_require_and_subpath_patterns` fail on base | Exact null subpath blocks a matching pattern; escaping target declines |
| Canonical package links and modern self-reference | Linked and self-reference native controls have missing baseline targets | Uninstalled discovery and legacy self-reference do not bind; external sibling shadow stays unresolved |
| Custom conditions | Matching and nonmatching native controls have missing baseline targets | Nonmatching condition falls to default |
| Paths precedence and ambient fences | Base test's linked-package positive fails; removing the final paths-refusal guard fails its module-proof assertion (PKG08) | Missing matched path retains refusal; unrelated ambient declaration permits binding, matching wildcard blocks it |
| Builtin/loader classification | New public API's head assertions test all four enum classes; the API does not exist on base | `node:` and loader schemes produce no repo target; relative `./node:url` remains relative |
| Changed cache facts | Full suite includes the pinned epoch assertions and existing cache tests | Earlier epochs invalidated; existing malformed/stale cache refusal checks run in full suite |

The new classification API cannot provide an executable behavioral comparison
against an absent pre-change API. Its test would fail to compile on base; that
compilation failure is **not counted as behavioral RED evidence**. Seven existing
path behavior tests supply actual base RED (`base-red.log`), and the mode repair
has its own wrong-target RED (`bundler-mode-red.log`). The scoped mutation witness
checks preservation of the existing paths refusal at the module-proof seam.

Other completed checks:

- Immediate same-worktree `cargo build --offline --release --bin prism`, then
  `/Users/wesleyjinks/code/slicing/eval/.venv/bin/python -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin /Users/wesleyjinks/code/prism-pkgres/target/release/prism --date 2026-10-04-pkg-v2`
  from `eval`: **178 OK / 0 regressions / 0 skips**. Rebuilt bytes equal the final
  pinned head binary (`matrix-v2-rebuild.log`, `tier-a-v2.log`).
- S1b-4 complete comparison: **411 unchanged scenarios, 1,234 output files
  byte-identical**, zero stderr (`s1b-v2-comparison.json`,
  `s1b-v2-byte-identity.json`).
- X, installed-X, R, T: complete streams byte-identical to main; zero changed,
  lost or unproven call rows. Installed-X gains one native-correct module proof
  (`public-v2/summary.json`, `public-v2-preservation.json`).
- Fresh S2 parent/overlay measurements: **0/132 recovered** on both X snapshots;
  zero lost/unproven rows (`s2-final/summary.json`).
- `bash -n docs/superpowers/plans/2026-10-04-workspace-package-resolution/CONTROLLER-pkg.sh`:
  **PASS**. Python probe AST and mutant JSON validation: **PASS**.

Production, test, binary and probe hashes are pinned in the packet's
`BUILD-MANIFEST.json`; snapshots and fixture links are recorded in
`planning/final-custody.json`. Temporary-example fmt/clippy setup failures were
inadmissible and corrected before the successful final permanent-source checks.

## Not verified

- Private corpus F: controller-only; never opened by this worker. The script was
  syntax-checked and its underlying native checker exercised on public controls.
- Independent review, full package/Node/TS conformance, remaining JS/config and
  discovery coverage, performance/adoption limits, and off-machine backup.
- Tier-A quick, explicitly skipped by the brief, and human-triggered full Tier-A
  corpus runs.
- Optional `detached-owner-audit`/all-features tests and the entire scratch S2
  suite. These were outside the requested MCP suite; they were not reported as
  executed or as environment failures.
- Six registry mutations were outside the advisory Git-diff selection because
  their module is untracked. The selected denominator is two, not eight.
- A final-source nextest repeat: the brief requested one nextest run; final
  source was instead checked by the full MCP Cargo suite after repairs.
- Isolated causal attribution from S2 diagnostic traces: repeated graph builds
  have no graph IDs. Receipts retain last-observed and conservative-union states;
  risk counts overlap. Actual complete-row recovery is independently zero.

This is a measured bounded prototype and planning packet. Broad feature
completion, any new refusal cut, virtual-loader policy, generated-output source
substitution and S2 adoption are not claimed. Git custody remains with the
controller. Build directories were pruned after validation; retained binaries,
source snapshots and verified receipts remain in the evidence root.
