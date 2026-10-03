# P1 repair r3 — PARKED, NOT SHIPPABLE

HEAD remains `247f1627a73866b01fb25c916cb38507588e5ba5`, branch `feat/tsconfig-paths-p1`, with the existing artifact repaired in place. No Git writes, installation/network, baseline change, private F access or agent delegation. Cache stays **105/61**. Both requested round-3 reviews and their probes were read fully. Current-only receipts are in `target/repair-r3/current`; earlier receipts cannot certify this source.

The acceptance claim is **REFUTED**: current installed X recovers **0**, and four additional closure mechanisms produce wrong Exact edges in both grammars. The supplied AGENTS.md convergence instruction says: “if findings are open-class (each round surfaces new instances of the same kind), park and escalate to spec or design.” Product edits stopped at that gate. The existing patch and pre-main checkpoint are preserved; there was no restart.

## Reviewer dispositions

| Review finding | Disposition on its inputs |
|---|---|
| Opus W1 / sol R3-W2 — ancestor automatic/named/reference types | Fixed on reviewer inputs: outside ancestor inputs warn, count and preserve base; explicit `types:[]` positive controls remain. |
| Opus W2 / sol R3-W1 — uppercase declarations and candidate collisions | Fixed on reviewer inputs: insensitive suffix scan; one root-volume probe with insensitive fallback; simple-fold occupancy for candidates, siblings, priority and first-pass probes. Pinned Unicode16 comparison: 3,034 directed pairs, zero mismatches after three missing simple mappings were corrected. |
| sol R3-W3 — declaration import outside root | Direct reviewer inputs fixed, including import/export/import-type/require/reference, absolute paths, package redirects and outside links. **Normative C remains OPEN** through the four witnesses below. |
| Opus S1 — installed traversal/budget | Traversal repaired: `.bin`/`.git` excluded, in-root links deduped by canonical path, outside links decline, source bytes replace entry count. **Yield remains FAILED** because ordinary-main closure now causes a blanket cut. |
| Opus S2 — M65 integration coverage | Fixed: the `util.jsx` + `UTIL.TS` row kills paired M65/I56 at integration level. The independent singleton gate and first-pass paths arm are disabled together to test the invariant. |

Current reviewer replay: **106 fixtures**, **84 base-preserved / 22 changed**, **61 native-valid / 45 diagnostic or inferred**, **0 incorrect certified targets**. First-pass controls: **146 occupied / 146 removed**, respectively **2 / 146 admitted**, zero wrong and zero unexplained native-JS refusals. These are finite populations, not a closure proof.

## Remaining WRONGs — concrete native witnesses

All four have `types:[]`, configured native owner `tsconfig.json`, no diagnostics, and implementation **Exact `src/util.tsx:real`**. Native instead selects the outside ambient declaration. Both JSX and TSX reproduce. Original base `dab8251c` preserves all eight rows. The executable at `247f1627` is bound to all **273** Git/source inputs and tested in the same environment: three mechanisms already fail there; the excluded `.bin` mechanism is newly admitted by this repair.

| WRONG / MATERIAL | Input and mechanism | Bounded repair seam |
|---|---|---|
| C1 source-chain | `types/local.d.ts` imports `./inner`; `types/inner.ts`, containing no `declare`, imports `../../outside/global`. Dependency extraction only retains declaration-bearing files, so it loses this transitive source edge. | Keep dependency edges for source inputs that can extend declaration reachability; the ambient `declare` prefilter cannot prune the dependency closure. |
| C2 paths-alias | Declaration imports `vendor`; effective `paths.vendor` points to `../outside/global.d.ts`. Boundary lookup checks packages but ignores effective config substitutions. | Check declaration dependencies using the effective config's paths/baseUrl and bounded Node10 resolution; memoize per config. |
| C3 ignored-bin | Declaration imports `../node_modules/.bin/global.d.ts`; `.bin` is a directory symlink outside the root. Traversal correctly skips it, but dependency lookup interprets the uncaptured prefix as absence. | Require a physical captured/inside proof before accepting a dependency; excluded or opaque occupancy is not proven absence. |
| C4 parent-capture | Declaration imports `virtual/pkg/../../outside/global`; `paths["virtual/*"]=["./*"]` substitutes a capture that normalizes outside. Bare package lookup never applies the substitution. | Validate the selected dependency substitution after capture and refuse normalization that escapes the retained root. |

Evidence: `current/witnesses/{source-chain,paths-alias,ignored-bin,parent-capture}{,-jsx}.json`, with input/link hashes, original-base/pre-change/current outputs and native program files/declarations. Native logical declaration for C3 is `node_modules/.bin/global.d.ts`, whose physical target is outside; the others name `../outside/global.d.ts`. Confidence **100/100** for these exact inputs: clean owner-bound declarations, physical target facts and same-environment controls raise confidence. Input/owner drift would lower it; preserving base or native binding the local implementation on unchanged bytes would collapse it. No inherited WRONG was downgraded.

## Installed-X failure population and scan measurement

The complete diagnostic enumerates **74 failing declaration-dependency requests**, caused by exactly two physically regular in-root targets: `node_modules/terser/bin/terser` (**444 bytes**, extensionless JavaScript) and `node_modules/next/dist/compiled/constants-browserify/constants.json` (**4,621 bytes**, JSON). Ordinary-main closure sends these through type-source coverage and labels them unsafe. Neither leaves the root. It declines **four configs**, removing all recovery. A genuine outside-main witness was fixed by the extension, so reverting that extension would reopen an observed WRONG; neither state is accepted.

Installed inventory: **16,060 TS-family files / 64,457,541 bytes**, **8,667 byte-declare candidates**; the conservative TS+JS reader covers **44,932 files / 390,756,660 bytes**, **36.392% of 1 GiB**. There is no directory-entry budget. All **16,060 TS input hashes** were reverified during current verification. **Nine** retained non-bin links all target in-root workspaces. Complete diagnostic traversal counted **68,759 entries**, which are informational and do not consume the bound.

Fresh current plain X: **3,121 changed, all CORRECT_STATIC_BINDING**, zero ownership disagreements. Installed X: **0 changed**, oracle executed against the installed tree, zero differences available to certify; **FAIL against >=3,100**. R and T: **0 changed each**, no warnings. Do not transfer the preceding source's installed gains to this patch.

| Corpus | Changed / oracle | Base/head wall s | Base/head RSS bytes |
|---|---|---:|---:|
| X | 3,121 / all CORRECT | 45.2494 / 53.5792 | 759,136,256 / 775,192,576 |
| Installed X | **0 / FAIL yield** | 33.2905 / 50.1396 | 757,202,944 / 795,099,136 |
| R | 0 | 0.5470 / 0.5436 | 66,600,960 / 66,437,120 |
| T | 0 | 253.1842 / 253.8522 | 3,392,208,896 / 3,356,442,624 |

The one declared same-environment pre-main attribution control uses immutable binary `ccec292d4a342c9f3cbcec16f5a328dc81a145c2d8ead49195703dbb3781604a` and the fresh installed base stream. It recovers **3,121, all CORRECT**, zero ownership disagreements, with **91.1587 s / 859,701,248 bytes RSS**, and no warnings. Thus the current ordinary-main extension's zero yield is confirmed against its predecessor on this installed tree. This control is not a performance retry or an accepted predecessor rollback. Receipt: `current/pre-main-installed-control/summary.json`.

Installed base/head wall: **33.2905 / 50.1396 s** (1.5061x); peak RSS: **757,202,944 / 795,099,136 bytes** (1.0500x). Stderr:

```text
warning: P1 paths config declined: outside-root or unread type input "terser" from "node_modules/terser/bin"
warning: P1 paths config declined: tsconfig.json: outside-root module dependency "terser" from node_modules/@rollup/plugin-terser/types/index.d.ts
warning: P1 paths config declined: examples/with-nextjs/tsconfig.json: outside-root module dependency "terser" from node_modules/@rollup/plugin-terser/types/index.d.ts
warning: P1 paths config declined: examples/with-script-in-browser/tsconfig.json: outside-root module dependency "terser" from node_modules/@rollup/plugin-terser/types/index.d.ts
warning: P1 paths config declined: packages/tsconfig.base.json: outside-root module dependency "terser" from node_modules/@rollup/plugin-terser/types/index.d.ts
P1 paths: absent type inputs=8 (distinct origin/input pairs)
P1 paths: boundary-declined configs=4
```

## Current verification

Full offline suites, with the existing pinned TypeScript 5.9.3 compiler: **default 4,917 / MCP 5,110 / all-features 5,133 passed**, **0 failed**, **1 existing ignore each**. Fmt and all-target/all-feature clippy pass with warnings; the duplicated-attributes diagnostic in the common test module also notes the shared integration main. No unrelated cleanup was applied. Immediate same-worktree release rebuild then Tier-A matrix: **170 OK / 0 regressions / 0 skips**. Existing Python3.12 CLI was used directly, without installing or changing the baseline.

**82/82 integration/unit mutants and 68/68 kernel mutants killed** on current source. Only behavioral assertion failures or resolver mismatches count; compile/setup/zero-test failures are inadmissible. Original populations retained. Two initial passes were declared; the additional ordinary-main/metadata two-arm extension was disclosed before dispatch, adding two arms to the integration denominator. No new product retry followed the four open-class witnesses.

New integration regressions run both grammars; the pre-change `247f1627` replay has **11 behavioral RED / 1 GREEN**. The GREEN is a pre-existing physical Unicode-absence guard, not a claimed newly fixed behavior. The added in-root `.git` alias edge also had a behavioral RED before its targeted correction. Initial zero-selected-test, incompatible-extern and launcher refusals are retained as inadmissible probes. The initial 21 unpinned all-feature failures were environment refusals; same-environment pre-change control had the same 21 failures and 1,453 passes. With the existing compiler pin, all full suites above pass.

Cache **105/61**: cross-binary rebuild/hit parity, **10 scanner cases / 80 add-edit-remove states**, and **12 reviewer cases / 24 directions** pass, including adding and removing a `node_modules` declaration. S1b-4: **411 controls**, both site and function streams byte-identical to original base, stderr **0 bytes**. Complete synthetic controls: **487 scenarios / 503 rows**, **0 recovered / 0 lost / 0 other changes versus r2d**; fresh r2d same-environment output equals its saved stream. Of 115 changes against original base, **111 correct bindings / 2 correct refusals / 2 parked S6/OQ2 UNPROVEN**.

Serial Nx uses three alternating base/head pairs per scenario, **18 measurement children**, each producing **64,000 sites** with expected Exact counts. Median results:

| Scenario | Base/head wall s | Wall ratio | Base/head RSS bytes | RSS ratio |
|---|---:|---:|---:|---:|
| nx | 6.2046 / 6.8782 | 1.1086 | 713,670,656 / 892,633,088 | **1.2508** |
| nx_bundler | 6.1860 / 6.4881 | 1.0488 | 714,440,704 / 761,544,704 | 1.0659 |
| nx_wild | 6.3358 / 7.0933 | 1.1195 | 723,910,656 / 895,057,920 | **1.2364** |

Wall ratios pass 1.20; nx and nx_wild RSS exceed it. Own measurement jobs were serial after verification jobs ended. Host quiescence is unverified; no exclusive source attribution is made for memory. No unplanned retry or threshold relaxation.

Not verified: private F (explicit exclusion), independent post-repair review, current-source Tier-A quick, human-triggered full multi-corpus Tier-A, Linux/case-sensitive execution, quiet-host attestation or concurrent-tree security. The 45 diagnostic/inferred reviewer fixtures are not native-certified. Earlier paired quick is baseline-invalid on both at C-method 4/6 and oracle error 1/15; it predates this source and is not a current quick-green claim. The existing ignored test was not forced. No current closure/yield completion claim is made.

## Files and controller custody messages

Product: `src/js_paths.rs`, `src/js_paths_snapshot.rs`, new `src/js_paths_boundary.rs`, new `src/js_paths_case_fold.rs`. Tests: new `tests/integration/js_paths_r3_test.rs`, `tests/integration/js_paths_repair_test.rs`, `tests/integration/main.rs`. Documentation: `SPEC.md`, `MEASUREMENTS.md`, `BUILD-MANIFEST.md`, `HANDOFF.md`, new `REPAIR-R3-HANDOFF.md`, this report. Starting dirty bytes and subsequent stable checkpoints are archived under `target/repair-r3`; the repairer made no commit.

Final local custody: `target/repair-r3/current/final-binding.json`, `owned-files.tar.gz`, `essential-receipts.tar.gz`, `custody-final.json`. Verify with `python3 target/repair-r3/current/verify_custody.py`. The source/doc snapshot and lean receipt archive preserve the parked artifact; this is local custody, not an external-backup claim.

Proposed messages for controller **WIP custody only**, not acceptance:

1. `wip(paths): preserve P1 r3 repository boundary repair at design gate`
2. `docs(paths): record r3 closure witnesses and failed yield/performance gates`

The owner decision is a bounded continuation on this artifact: settle config-sensitive dependency closure, transitive source edges, excluded-path physical occupancy and ordinary-module non-type main treatment; require installed yield and the unchanged performance bounds before acceptance. The supplied cap rule requires that decision before another repair iteration.
