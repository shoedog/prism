# Lane-P P1 adopted performance fix — full verification PASS

[MEASURED] Checkout `/Users/wesleyjinks/code/prism-paths-impl`, branch `feat/tsconfig-paths-p1`, HEAD `b28f6e721552c946fab8a41039e7b30552fc7f41`. One full serial foreground pass, with setup-only repairs and the declared driver-preparation extension. No production/test/vendor/build input changed: 747 inputs bound in `target/verify-fable/current/source-binding.json`. No Git writes, providers/network or F access. Release binary `target/verify-fable/current/head/prism`, SHA-256 **6d88f31a6297445d3dc63ee7c6845eb87ce82d104b0f392c231188d9b1b59da0**. Source and binaries are frozen separately from verification-only driver/docs changes.

## Formatting, lint and suites

Fmt --check passed. Offline all-targets/all-features clippy: 0 new touched-file warnings; 22 diagnostics at HEAD and 22 at same-environment r4. Library/library-test warning summaries 139/182 on both. Existing warnings retained. Both clippy processes returned 0; source-bound diagnostics, rather than exit status alone, establish the warning result. Rust/clippy 1.94.0; dev debug=0 and incremental disabled. Same-environment r4 control was extracted with git archive b5e9ed72; no Git writes.

| Suite | Passed | Failed | Ignored |
|---|---:|---:|---:|
| default | 4924 | 0 | 1 |
| mcp | 5117 | 0 | 1 |
| all-features | 5140 | 0 | 1 |

All three suites used `PRISM_TYPESCRIPT=/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`, freshly SHA-256 checked as **3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675**. The one existing ignore is `resolution_test::slice_elem_variant_reserved`. No suite exclusions or test failures.

Immediate same-worktree release rebuild then Tier-A matrix-only: **170 OK / 0 regressions / 0 skips**. Pinned --sut-bin plus --allow-stale-sut used only immediately after that rebuild. No baseline change. Logs and totals: `current/logs/`, suite JSONs and matrix-summary.json.

## Mutants and driver staleness

| Mutants | Run | Killed | Survivors/inadmissible |
|---|---:|---:|---:|
| Kernel | 67 | 67 | 0 |
| Integration/library | 93 | 93 | 0 |
| Resource | 2 | 2 | 0 |

All **53** distinct behavioral selectors passed on the isolated unmutated adopted source before mutations. Kills require exactly one selected test and a real behavioral panic, or a nonempty kernel output difference; compile/setup/zero-selection failures do not count. Isolated source restoration was checked after each mutation. Resource kills require identical output and median RSS above the existing 1.20x regression bound.

- `P01-eager-iterator`: RSS ratio to identical-output reference **1.364944x**; KILLED; 3 alternating pairs, byte-identical 64,000-site dumps.
- `P02-eager-cli`: RSS ratio to identical-output reference **1.363257x**; KILLED; 3 alternating pairs, byte-identical 64,000-site dumps.

Fable made **8 literal anchors** and **9 injected return shapes** stale in the 93-item integration/library population. Rebound only driver injections, preserving mutant IDs, test selectors and invariant intent: `I18-no-ambient-content-hash`, `I19-no-ambient-read-occupancy`, `I24-no-reference-closure`, `I35-no-installed-ambient`, `I72-large-source-bound`, `I74-tolerant-literal-exclusion`, `I82-entry-budget`, `R4-01-unread-metadata-declines`, `R4-02-outside-link-declines`, `R4-03-non-source-declines`, `R4-04-non-utf8-declines`, `R4-08-dropped-import-decline`, `R4-11-unread-file-declines`, `R4-12-unread-dir-declines`, `R4-13-non-regular-declines`, `R4-14-non-utf8-path-declines`, `R4-20-duplicate-indexed-read-receipt`. Kernel `M41-no-source-ambient-scan` moved to the parsed-result replay arm; its driver also needs the new rayon extern. The historical **M43 kernel + 12 integration retirements** remain explicitly excluded by r4 owner policy; no new retirement or denominator reduction. Reusable packet drivers are `probes/mutants.py`, `probes/integration_mutants.py`, `probes/adopted-mutants.json`; exact rebindings and hashes in current/driver-rebindings.json and driver-binding.json.

Two preparation spelling errors (literal/next and refs/references) were inadmissible setup observations, corrected against exact source; the initial two-attempt preparation cap was extended once and disclosed. No behavioral gate retry or production fix. All kernel/integration/resource receipts retained under current/.

## Cache and preservation controls

Cache 105/61: packet probe **20 query artifacts**, scanner **10 cases / 80 paired cached/fresh states**, reviewer **12 cases / 24 add/remove directions**, tolerant-policy **10 cases / 40 paired states** all pass. Explicit node_modules package/@types source add, same-pattern edit, nonmatch edit and removal are covered. Old-binary rebuild, cold/hit/fresh parity, config/candidate/extends/package changes, unrelated text stable hits and link/UTF8/malformed transitions pass. Receipts: current/cache-base, scan-cache, cache-review, cache-tolerant summaries and logs.

Packet controls versus freshly executed r4: **487 scenarios / 503 rows**, **0 row or stderr differences**. S1b-4 versus base: **411 controls / 639 sites**, **822 site/function byte comparisons**, **0 differences / 0 stderr bytes**.

The first control/S1b dispatch found removed local fixture directories and made no behavioral query. These setup observations are inadmissible, retained in setup-failures.json. Exported fixtures were restored byte-for-byte at their original literal paths (347,318 bytes, full per-file binding in fixtures-restored.json); these two checks then ran serially to completion. Fixtures were never rewritten; original absolute-path semantics retained.

## Corpus oracle and performance

| Corpus | Changed rows | CORRECT bindings | New wrong/unproven |
|---|---:|---:|---:|
| X | 3121 | 3121 | 0 |
| installed-X | 3121 | 3121 | 0 |
| R | 0 | 0 | 0 |
| T | 0 | 0 | 0 |

Each corpus used fresh base/head dumps, import extraction and the offline native oracle. No added/removed site keys. Plain and installed X have identical base dumps, identical head dumps and identical import facts. R and T base/head raw dumps are byte-identical. Counts certify changed rows, not whole-corpus correctness. Oracle and stream hashes, changed rows and classifications retained; transient full streams removed after hashing. T base/head measured 248.298409/249.201663 seconds in this pass (row check only).

**Performance gates: wall <=1.30x; maximum RSS <=1.20x.** Three alternating base/head pairs per scenario, 24 serial timed children total; no own verification job overlapped. MacOS getrusage child maximum RSS in bytes, fresh measurement process per child. Medians:

| Scenario | Base/head median wall s | Wall ratio | Base/head median RSS bytes | RSS ratio | Gate |
|---|---:|---:|---:|---:|---|
| installed-X | 35.627627 / 40.427853 | 1.134733x | 758628352 / 868728832 | 1.145131x | PASS |
| nx | 6.442015 / 6.681905 | 1.037238x | 765673472 / 656850944 | 0.857873x | PASS |
| nx_bundler | 6.523864 / 6.425766 | 0.984963x | 768180224 / 644268032 | 0.838694x | PASS |
| nx_wild | 6.596521 / 6.870606 | 1.041550x | 764690432 / 668450816 | 0.874146x | PASS |

Each Nx run has **64,000 sites**. Exact counts: head exact/wildcard **64,000**, bundler **0**, base **0** in all scenarios. Nx dump bytes match the retained r4 reference hashes. One-minute host load across timed children **3.532–7.455**; no quiet-host or global process-census attestation. Load is an observation, not an explanation attributed to the patch. Run commands, CPU times, stderr, hashes and load observations are in current/perf/ and checks-summary.json.

## Failures, pointer, custody and exclusions

None. All requested hard gates passed.

Controller `probes/CONTROLLER-paths.sh` defaults to the pinned new binary and is syntax-checked only; no F execution. Prior reference-base binary SHA-256 **d5b3a84ff9b10fcc8fbda4b7050a625bdb10f3ef7b9eb27ffca9ac4e88151aa6**, r4 **79b00a7c85938f7aafdae22bbb2321a47e1d67c307fbe5c8814a67a1b05b34f6**, and base import extractor **fd77e531437d4907313ffd98d11f31d5289aecd0854ca60b72447de010f5a2ca** were rehashed and executed locally. Their source/build provenance is inherited from supplied Fable/r4/r1 receipts; their release binaries were not freshly rebuilt here. The r4 source clippy control was freshly run.

Not verified: **F/private oracle or controller F execution; Tier-A quick/full multi-corpus; quiet host/global process census; fresh reference-base/r4 release builds.** Every requested non-F verification check completed. The owner-accepted ambient-shadowing cost is preserved; this pass does not replace normative policy or certify unrequested cases.

Lean receipts and pinned head binary remain in `target/verify-fable/current/`; source/docs/receipt snapshots are local custody, not a pushed or external backup. Disposable build, extracted/cloned source and restored fixture directories are removed at the end and enumerated in cleanup-summary.json. Source input parity, pointer binding, final docs and receipt hashes are in custody-final.json. Controller owns Git custody; see [VERIFY-FABLE-FILES.md](VERIFY-FABLE-FILES.md).
