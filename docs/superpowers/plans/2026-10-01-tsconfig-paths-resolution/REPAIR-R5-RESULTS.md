# Lane P P1 r5 — targeted repair verified locally

[MEASURED] Branch `feat/tsconfig-paths-p1`, HEAD `5aa3055218b3be250ce3fef742c494f8f7a5b5a8` plus the dirty r5 fold. No repairer Git writes or F access. Final binary `target/repair-r5/head/prism`, SHA-256 `907d110c70962063d5fde23acd22b6d92b62b117d837b97a81f6d65ed263aa03`; source binding `ddd3a9c4d844a180a7b43ebda7db8daef23265ccccd9e37919806b4d9d04fbad` across **1020 inputs**. Cache stays **105/61**. The controller wrapper defaults to this binary; only its shell syntax was checked, never the private run.

## Implementation and RED/GREEN evidence

- The ambient scanner ports pinned TypeScript 5.9.3 `sys.readFile`, `typescript.js:8525–8549`, SHA-256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`: UTF-16LE/BE BOM decoding, UTF-8 BOM removal, otherwise lossy UTF-8. Hashes and budget counters bind raw bytes. Both grammars cover Latin-1 U1, UTF-16LE U2, UTF-16BE, UTF-8 and UTF-8 BOM, plus nonambient controls. The prior invalid-byte skip assertion now requires a counted lossy read. Rust replaces unpaired UTF-16 surrogates; Node strings retain them. This representational distinction does not change the ASCII declaration keyword; it is not a claim of JavaScript string identity.
- Source eligibility and relative references use each eligible link's own name. Target bytes are read; aliases share a physical read/hash/parse within each traversal. Canonical type-input closure also visits captured lexical alias references. Existing alias-count coverage now checks all three reference names. The cross-directory `.git/link.d.ts` control was WRONG before the fold: native ProjectService/checker bound `.git/guard.d.ts`, with no diagnostics, while the preceding binary returned implementation Exact. The original base retained UnknownName. Positive and empty-source controls now pass.
- Explicit/reached in-repo sources under `.git` and `node_modules/.bin` are scanned. Config hints cover `files`, literal include prefixes, wildcard-name and recursive patterns with explicit hidden segments, relative type inputs, roots, path references and metadata redirects. Ordinary wildcard inclusion keeps unused tooling excluded. `.g*/**/*` was WRONG before the targeted fold: native loaded `.git/guard.d.ts`, while the preceding binary returned Exact. The existing test covers seven activation modes in two directories and both grammars, with unloaded and emptied-source controls. Conservative hint overapproximation may scan more than native membership; unsupported type inputs still preserve base by declining configs.
- Aggregate stat sizes reserve the remaining byte budget before parallel reads. Reads stop at their reservation and check growth with one stack byte. Large reads/decodes/parses serialize. Tests cover exact capacity, aggregate overflow, growth, unreadable jobs and decoder edges. The same sparse over-budget fixture demonstrates the old memory cost and early rejection below.
- The memo implementation remains keyed by `(input, roots)`. A new two-root test observes safe/unsafe closure results in both orders and repeated queries; the roots-ignoring mutant fails.

[INHERITED] Initial RED receipts (`red-decoding.log`, `red-link.log`, `red-loaded.log`) show three integration tests failing behaviorally against unchanged r4 production; those initial RED executions were not repeated in the final resumption. [MEASURED] Final suites, exact selector baselines and mutants were rerun against the frozen source. The memo test is a regression shield, not a fix to a currently broken memo. H3/H4 controls and their original-base/native discriminators are retained. The native include audit has ten observations: TypeScript excludes wildcarded `node_*`, while literal `node_modules/.b*` and recursive literal `.bin` load the declaration. The Rust regression also exercises the conservative wildcarded-node overapproximation; native positive fixtures use the literal directory. The initial native-witness assumption was inadmissible and corrected before retry. The new reference-cache fixture initially omitted its guard: cached and fresh both correctly declined the unread input. An empty guard restored Exact; the corrected cache cycle edits that guard between empty and ambient, preserving the type-input rule. Compiler/setup/zero-selection failures are inadmissible, never counted as evidence or kills. `hypothesis-probe-result.log` preserves the control results and corrections. This is the disclosed final targeted extension of the existing artifact, not a new review round or an artifact restart.

The previous frozen r5 batch missed installed-X RSS: **1.202040551x** against **1.20x**, while wall **1.271940021x** passed. Its binary, source and receipts are retained in `allocation-before/`. A stat/source-job census counted **44,932** ordinary jobs and **3,673,752 bytes** of duplicate String/Vec-element storage before allocator rounding. The bounded fold stores the canonical name once and allocates alias names only for distinct links. This proves the removed allocation population; parser scheduling remains an alternative explanation for some RSS variation. The first final-suite attempt used a raw `/var/...` tempfile root in a new internal-traversal assertion, although production canonicalizes to `/private/var/...`. The path-identity probe demonstrated that both links failed the raw-root boundary check; canonicalizing only the test root gives **1/1 GREEN** with all three references. This was an inadmissible setup failure, not a production regression. Retained in `inadmissible-alias-test-root/`; corrected serial batch is setup attempt **2/3**. No resource-gate denominator or threshold changed.

## Public yields

Counts classify changed rows only, not whole-corpus correctness. Fresh final streams are byte-identical to r4, with no site-key additions/removals. X and installed-X receive fresh offline native certification; R/T have zero changed rows, live certified input hashes and full byte parity, so there is no new row to classify.

| Corpus | Sites | Changed vs original base | Correct static bindings | Wrong / unproven new rows |
|---|---:|---:|---:|---:|
| X | 19,219 | 3,121 | 3,121 | 0 / 0 |
| installed-X | 19,219 | 3,121 | 3,121 | 0 / 0 |
| R | 953 | 0 | 0 | 0 / 0 |
| T | 61,712 | 0 | 0 | 0 / 0 |

Receipts: `yield/summary.json`, compressed complete streams, changed-row files, fresh native certificates and import facts. Original comparison binary SHA-256 `8722d1afed5fd66892ee1c8cc6b00c397301af7d8cb8e61263cf5875fdc040e7`; reviewed r4 binary `1a9dc5902e52c66416b16b747efdd2fd4b5cf0b5ddd031a53c39a05ceb2f3a89`. Both pinned binaries ran in this environment. Original dab8251c source custody is inherited from the retained lane receipts; its binary was not freshly rebuilt here.

## Controls versus r4

**487 scenarios / 503 sites:** zero stdout or stderr changes. **108 retained review cases:** exactly 8 row changes, all full original-base rows restored; every other row remains identical.

| Changed case | Explanation |
|---|---|
| `scan-bin-explicit-jsx` | explicit excluded root now scanned; native-loaded matching declaration retains full base |
| `scan-bin-explicit-tsx` | explicit excluded root now scanned; native-loaded matching declaration retains full base |
| `scan-git-explicit-jsx` | explicit excluded root now scanned; native-loaded matching declaration retains full base |
| `scan-git-explicit-tsx` | explicit excluded root now scanned; native-loaded matching declaration retains full base |
| `scan-link-text-jsx` | lexical source link now scanned; native-loaded matching declaration retains full base |
| `scan-link-text-tsx` | lexical source link now scanned; native-loaded matching declaration retains full base |
| `scan-utf16-le-jsx` | native UTF-16LE decoding; native-loaded matching declaration retains full base |
| `scan-utf16-le-tsx` | native UTF-16LE decoding; native-loaded matching declaration retains full base |

Only the two UTF-16LE review cases change stderr: the obsolete `non_utf8_source` skip/warning disappears. No other control stderr changes. The sparse budget control changes refusal counters because reservation now rejects before reads; its stdout remains identical.

Fresh native witnesses: **42 positive + 42 empty-source negative controls**, two grammars; owning config/checker proven and positive diagnostics empty. **S1b-4: 411 controls / 639 sites / 822 site-and-function comparisons**, byte-identical to r4, **0 stderr bytes**. Old C80/OQ2 ownership questions and the accepted outside/transitive ambient cost are preserved, not newly certified.

## Suites, lint, accuracy, mutations and cache

| Suite | Passed | Failed | Ignored |
|---|---:|---:|---:|
| default | 4,929 | 0 | 1 |
| mcp | 5,122 | 0 | 1 |
| all-features | 5,145 | 0 | 1 |

All requested suites ran offline; no suite excluded, no filters. One existing ignore each: `resolution_test::slice_elem_variant_reserved`. Fmt passes. Offline all-targets/all-features clippy: **372 r4 warning emissions / 372 final emissions / 0 new / 0 removed**. Baseline source is `git archive HEAD` in an isolated build directory in this same environment; diagnostics compare code, message, file and source text independently of line-number shifts.

Immediate release rebuild followed by Tier-A matrix: **170 OK / 0 regressions / 0 skips**. The installed CLI ran via its existing Python interpreter; no install/network or baseline changes. Current quick/full multi-corpus Tier-A was not run.

**Mutants: 26/26 killed, 26 admissible; 9 exact green baseline selectors.** The isolated driver uses incremental builds. Only touched-code variants and the five new tests' variants ran; the prior full mutation matrix is not a current claim.

| Mutant | Disposition | Test |
|---|---|---|
| `I18-no-ambient-content-hash` | behavioral kill | `js_paths_cap_test::structural_scan_dependency_add_remove_edit` |
| `I78-canonical-dedupe` | behavioral kill | `js_paths_snapshot::boundary::tests::linked_source_is_scanned_and_counted_once` |
| `I79-byte-budget` | behavioral kill | `js_paths_snapshot::tests::scan_budgets_disable_repository` |
| `R5-01-strict-utf8` | behavioral kill | `js_paths_r3_test::native_decoding_keeps_ambient_in_both_grammars` |
| `R5-02-ignore-utf16` | behavioral kill | `js_paths_r3_test::native_decoding_keeps_ambient_in_both_grammars` |
| `R5-03-wrong-byte-order` | behavioral kill | `js_paths_r3_test::native_decoding_keeps_ambient_in_both_grammars` |
| `R5-04-retain-utf8-bom` | behavioral kill | `js_paths_snapshot::boundary::tests::source_reservations_precede_reads_and_detect_growth` |
| `R5-05-raw-declare-prefilter` | behavioral kill | `js_paths_r3_test::native_decoding_keeps_ambient_in_both_grammars` |
| `R5-06-target-suffix` | behavioral kill | `js_paths_r3_test::lexical_source_link_to_text_keeps_ambient_in_both_grammars` |
| `R5-07-skip-in-root-link` | behavioral kill | `js_paths_r3_test::lexical_source_link_to_text_keeps_ambient_in_both_grammars` |
| `R5-08-drop-loaded-scan` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-09-exclude-loaded` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-10-drop-files` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-11-drop-include` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-12-drop-type-hints` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-13-drop-reference-hints` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-14-ignore-reservation` | behavioral kill | `js_paths_snapshot::boundary::tests::source_reservations_precede_reads_and_detect_growth` |
| `R5-15-reserve-individually` | behavioral kill | `js_paths_snapshot::boundary::tests::source_reservations_precede_reads_and_detect_growth` |
| `R5-16-ignore-growth` | behavioral kill | `js_paths_snapshot::boundary::tests::source_reservations_precede_reads_and_detect_growth` |
| `R5-17-memo-ignores-roots` | behavioral kill | `js_paths_snapshot::tests::type_closure_memo_separates_roots_with_different_results` |
| `R5-18-physical-reference-base` | behavioral kill | `js_paths_r3_test::lexical_source_link_to_text_keeps_ambient_in_both_grammars` |
| `R5-19-omit-link-reference-closure` | behavioral kill | `js_paths_r3_test::lexical_source_link_to_text_keeps_ambient_in_both_grammars` |
| `R5-20-discard-deduped-alias` | behavioral kill | `js_paths_snapshot::boundary::tests::linked_source_is_scanned_and_counted_once` |
| `I24-no-reference-closure` | behavioral kill | `js_paths_r2b_test::transitive_references_and_cycles` |
| `R5-21-ignore-explicit-hidden-pattern` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |
| `R5-22-drop-skipped-path-inventory` | behavioral kill | `js_paths_r3_test::loaded_excluded_sources_keep_ambient_in_both_grammars` |

Manifest and exact selectors: `mutations.json`; complete diagnostics, source hashes and behavioral outcomes: `mutants-summary.json` and `mutant-*.log`. I79 intentionally disables both budget guards.

Cache **105/61** passes **18 new cases / 72 states**, including cross-binary old-cache rebuild, warm hit bytes/mtime, unrelated-text stable hits, add/remove parity, lexical references and explicit hidden globs. Existing config/candidate/extends/package packet passes; existing scanner packet passes **8 cases / 64 states**.

## Serial resource gates

Three alternating repeats per binary/scenario; own producer commands ended before measurement. **24 serial timed children**, no own verification overlap. Peak RSS is macOS child `getrusage` bytes. No quiet-host attestation. Medians and limits:

| Scenario | Base/final wall seconds | Wall ratio / limit | Base/final RSS bytes | RSS ratio / limit |
|---|---:|---:|---:|---:|
| installed-X | 42.846264 / 52.484317 | 1.224945 / 1.30 | 756,973,568 / 830,095,360 | 1.096598 / 1.20 |
| nx | 7.203704 / 8.614939 | 1.195904 / 1.20 | 726,876,160 / 611,467,264 | 0.841226 / 1.20 |
| nx_bundler | 7.429503 / 8.125650 | 1.093700 / 1.20 | 713,605,120 / 607,715,328 | 0.851613 / 1.20 |
| nx_wild | 8.043679 / 8.362915 | 1.039688 / 1.20 | 713,474,048 / 628,178,944 | 0.880451 / 1.20 |

All raw repeats (order alternates base→final, final→base, base→final):

| Scenario | Repeat | Base/final wall seconds | Base/final RSS bytes |
|---|---:|---:|---:|
| installed-X | 1 | 42.846264000 / 57.756260958 | 756,973,568 / 830,095,360 |
| installed-X | 2 | 41.053979750 / 52.331658875 | 753,664,000 / 865,452,032 |
| installed-X | 3 | 43.448095667 / 52.484317209 | 757,628,928 / 761,348,096 |
| nx | 1 | 8.044262583 / 9.023614917 | 715,096,064 / 611,467,264 |
| nx | 2 | 7.203703833 / 8.299257583 | 726,876,160 / 626,737,152 |
| nx | 3 | 7.183938625 / 8.614939000 | 726,990,848 / 585,121,792 |
| nx_bundler | 1 | 8.698987375 / 8.567518083 | 721,879,040 / 607,715,328 |
| nx_bundler | 2 | 7.429503125 / 8.065382708 | 713,146,368 / 610,009,088 |
| nx_bundler | 3 | 7.409585459 / 8.125649875 | 713,605,120 / 607,256,576 |
| nx_wild | 1 | 8.398084791 / 8.362914875 | 714,326,016 / 625,000,448 |
| nx_wild | 2 | 7.869451167 / 7.842677333 | 695,484,416 / 628,178,944 |
| nx_wild | 3 | 8.043679042 / 8.662926792 | 713,474,048 / 630,341,632 |

`performance/summary.json` binds each run to binary/output hashes and sites: installed-X **19,219**, each Nx variant **64,000**. All output checks pass.

Sparse budget control: `budget/receipt.json` records two **1,073,741,825-byte** sparse source jobs, **0 physical blocks**, two workers. Full stdout parity; r4 buffers the over-budget files before refusal, while final refuses the reservation before reads. Raw measurements:

```json
{
  "r4": {
    "status": 0,
    "wall_seconds": 1.882474541,
    "peak_rss_bytes": 2179366912,
    "user_seconds": 0.971413,
    "sys_seconds": 1.148796,
    "stderr": "warning: P1 paths disabled: P1 ambient scan byte budget (7 scan entries, 33 bytes)\nwarning: P1 paths tolerant scan skipped: {\"non_source\":1}\n",
    "rows_sha256": "2e43169cb4b020e65e5e0dff5d1b36d7c74b816331c74a427134f8a35da8cee9"
  },
  "head": {
    "status": 0,
    "wall_seconds": 0.05863120799999999,
    "peak_rss_bytes": 32227328,
    "user_seconds": 0.050477999999999995,
    "sys_seconds": 0.005194,
    "stderr": "warning: P1 paths disabled: P1 ambient scan byte budget (0 scan entries, 0 bytes)\n",
    "rows_sha256": "2e43169cb4b020e65e5e0dff5d1b36d7c74b816331c74a427134f8a35da8cee9"
  },
  "sparse_sizes": [
    1073741825,
    1073741825
  ],
  "sparse_allocated_blocks": 0,
  "deleted": true,
  "output_identical": true,
  "threads": 2
}
```

Supplemental populated-cache control: **18 already-wrong r4 caches / 18 repaired cached outputs / 18 stable warm hits**, with declaration bytes unchanged across binary queries. The original cache cycle started without the declaration and could not establish this stronger property; `cache-old-present-summary.json` supplies the additional proof. No source change was needed.

## Custody and limits

Source/test snapshot, owned-file snapshot/diff, final binary, source/receipt hashes and lean receipt archive are retained locally under `target/repair-r5`. The r5 build directories and isolated source copies were removed after their producers ended; exact paths and retention checks are in `cleanup.json`. Duplicate loose native intermediate arrays/import facts were removed only after verifying their archived bytes; historical executables are retained losslessly as `prism.gz`. `lean-cleanup.json` binds those bytes. The complete compressed streams, final certificates, summaries and current executable remain directly available. The Git inventory is 11 files plus the ignored local `VERIFICATION.md` summary. These are local snapshots, not a pushed backup. Pre-fold receipts remain explicitly historical.

Not verified: F/private oracle (never opened), independent review, quiet-host conditions, current Tier-A quick or full multi-corpus, Linux/case-sensitive behavior or concurrent-tree semantics, the existing ignored test, the prior full mutation matrix, fresh rebuilding of the original comparison base, commit/push/deployment. The owner-accepted outside-only or unscanned-transitive ambient shadowing cost remains. Controller Git custody remains open; proposed commits and the exact owned inventory are in [REPAIR-R5-FILES.md](REPAIR-R5-FILES.md).
