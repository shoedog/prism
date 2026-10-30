# Lane-P P1 r4 — local accepted-risk repair verified

[MEASURED] Entry/current HEAD `e8da66f92ea48356cde2cf464dc729e7fb796e39`, dirty repair; no Git writes, network, F access or independent-review claim. Final binary `target/repair-r4/head/prism`, SHA-256 `79b00a7c85938f7aafdae22bbb2321a47e1d67c307fbe5c8814a67a1b05b34f6`. The controller wrapper points to this binary. Cache versions remain **105/61**.

The tolerant scan skips unreadable/non-source/non-regular/non-UTF8 files, unsupported declaration occurrences and outside-root links, counts reasons and emits one summary warning. It follows in-root links once per canonical path, excludes `.git` and `node_modules/.bin`, and still declines at the 1 GiB source-byte bound; package metadata remains separately bounded. Global ancestor-ambient and ordinary-import closure declines are removed. Valid in-root matching declarations, JS Node10 first-pass, physical case collisions, case-insensitive suffixes and explicit type-input fences remain. Malformed type-metadata siblings no longer hide valid outside redirects. One both-grammar test documents the owner's accepted outside-only/transitive ambient-shadowing cost without closing it.

Alias export projection retains only queried roots while preserving the full raw recursive proof and legacy export table. The RSS diagnostic then located the larger peak in the eager JSONL dump. CLI output now consumes one record at a time; the existing Vec API remains available. An empty/partial stream control preserves source-before-Go-candidate ordering and both accepted/dropped candidates. All 64,000-row Nx streams remain byte-identical.

## Yield and oracle gates

| Corpus | Changed rows | Oracle correct bindings | Wrong / unproven new rows |
|---|---:|---:|---:|
| X | 3,121 | 3,121 | 0 / 0 |
| installed-X | 3,121 | 3,121 | 0 / 0 |
| R | 0 | 0 | 0 / 0 |
| T | 0 | 0 | 0 / 0 |

Counts refer to changed rows, not whole-corpus correctness. No added/removed site keys. Native oracle is offline TypeScript 5.9.3, SHA-256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. Fresh streams, import facts, classifications and source/binary bindings live under `target/repair-r4/yield/` and `final-binding.json`.

Installed-X skip counters: **excluded 99; non_source 14,867; unreadable 0; outside_root_symlink 0; unparseable_declare_module 0; non_regular_source 0; non_utf8_source 0; non_utf8_path 0**. Absent explicit type inputs: installed X **16**, plain X **14**. Plain X skips **598 non_source**; R **30**; T **34**. Other counters zero.

Installed X base/head: **36.302570 / 101.864217 s** (**2.805978x**); RSS **758,857,728 / 936,050,688 bytes** (**1.233500x**). Installed resource ratios are reported, not a 1.20 acceptance gate.

## Serial Nx performance

Three alternating repeats per binary/scenario, 18 serial timed children, no own verification jobs overlapping. Peak RSS is macOS getrusage child bytes; medians shown. No quiet-host attestation.

| Scenario | Base/head wall seconds | Wall ratio | Base/head RSS bytes | RSS ratio |
|---|---:|---:|---:|---:|
| nx | 6.623871 / 7.818217 | 1.180309x | 715,751,424 / 641,433,600 | 0.896168x |
| nx_bundler | 6.787342 / 6.696157 | 0.986565x | 714,997,760 / 616,579,072 | 0.862351x |
| nx_wild | 6.864149 / 7.080579 | 1.031531x | 756,203,520 / 643,137,536 | 0.850482x |

Each scenario has **64,000 sites**. Head Exact counts: nx **64,000**, wildcard **64,000**, Bundler **0**; base **0** throughout. All six ratios pass **1.20**.

The initial three-round cap was reached. Bounded inventory compactions and root projection were retained on the existing artifact, with extensions disclosed; they did not explain the peak. The subsequent diagnostic/fix cap was one pass plus one measured fix. Trace: Nx context **640,417,792** bytes, accumulated dump **893,304,832**; Bundler context **611,663,872**, dump **768,950,272**. Pre-streaming serial RSS was **1.232198x / 1.221767x** on nx/wild, preserved in `projection-perf-failed.json`. The streaming fix is backed by byte parity and the eager-buffer mutants; no artifact restart, threshold/baseline change or unexplained host attribution.

## Controls, suites and mutants

Synthetic controls: **487 scenarios / 503 rows**. Versus r2d: **14 recovered / 0 lost / 0 other**. Versus r3: **14 changed, 14 recovered / 0 lost**. All 14 recovered rows have native correct bindings. The cumulative original-base census is **129 changes: 125 CORRECT_STATIC_BINDING, 2 CORRECT_STATIC_REFUSAL, 2 pre-existing C80/OQ2 UNPROVEN**; zero preservation violations. The two old ownership questions remain separately labeled and are not new r4 certifications.

| Check | Passed | Failed | Ignored |
|---|---:|---:|---:|
| default | 4,923 | 0 | 1 |
| mcp | 5,116 | 0 | 1 |
| all-features | 5,139 | 0 | 1 |

Fmt and offline all-targets/all-features clippy pass; existing warnings retained. No suite excluded. The one existing ignore is `resolution_test::slice_elem_variant_reserved`. Focused coverage retains **115 paths integration tests**, **15 snapshot units**, and projection/stream controls. Seven actual same-environment HEAD production controls are behaviorally RED; fixture/preflight failures were inadmissible and corrected before belief updates. The stream regression additionally has the pre-streaming serial resource RED above.

Mutants: **93/93 integration/library**, **67/67 kernel**, **2/2 resource** killed. The 12 r3 closure integration mutants and one malformed fail-closed kernel mutant are explicitly retired under the new owner policy, not counted as kills. Twenty-three tolerant-policy/projection arms were added; two eager-buffer mutants preserve exact row bytes while failing the bounded RSS regression against the identical-output repaired binary. Compile/setup/zero-selection failures do not count as kills. Immutable source hashes and exact selectors accompany the receipts.

Cache **105/61** passes: old-binary rebuild/cold/hit/fresh parity, config/candidate/extends/package edits; **10 scanner cases / 80 states**, **12 reviewer cases / 24 directions**, **10 tolerant cases / 40 states**. Add/edit/remove, inside/outside link transitions, malformed/invalid-UTF8-to-valid declarations and unrelated-text stable hits are covered. Skip warnings occur once per capture. S1b-4 **411 controls** retain byte-identical site and function outputs and **0 stderr bytes** versus base.

Immediate release rebuild then Tier-A matrix: **170 OK / 0 regressions / 0 skips**. Paired quick: **28 identical successful SUT outputs / 28 stored probes**, zero SUT or pinned-output differences. base: baseline_invalid **True**, reasons `['stratum C-method: 4/6 successful probes']`, oracle error rate **0.066667**, SUT error rate **0.000000**. head: baseline_invalid **True**, reasons `['stratum C-method: 4/6 successful probes']`, oracle error rate **0.066667**, SUT error rate **0.000000**. This is paired parity evidence, not a green quick baseline. No re-baselining.

Pinned quick dispositions for the controller PR: `target-c-method` expected `known_fail`, observed `flip_candidate`; `module-deps-feature-gated` expected `oracle_miss_site`, observed `missing`; `load-repo-feature-gated` expected `oracle_miss_site`, observed `missing`; `ambiguous-symbol-contract` expected `ambiguous_symbol_error`, observed `ok`.

## Custody and limits

`source-frozen.tar.gz`, `build-inputs-frozen.json`, `verification-binding-final.json`, `final-binding.json`, `custody-final.json` and the lean receipt archive bind source/tests/binaries/oracle and local results. This is a local snapshot, not a pushed backup. The reference-base binary was pinned and executed locally in every comparison; its dab8251c source custody is inherited from r3/base-binding.json (298 inputs), not freshly rebuilt in this turn. Its embedded version stamp is 7f5a862afd00-dirty. No F/private oracle, independent review, quiet-host attestation, full multi-corpus Tier-A, commit, push or deployment was performed. The owner-accepted outside-only/unscanned-transitive ambient shadowing cost remains. See REPAIR-R4-FILES.md for all owned paths and suggested commit messages.
