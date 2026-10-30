# Merge resolution: `github/main` into `tier-a-reanchor`

- Ours: `077e9129f73446da754192f982a1ba8c0bb49431`
- Theirs (`github/main`): `2c198f5fc85df698828364c913aa3730fe78e513`
- Merge base: `afc78147fd88feb60c06cc5dae9a8a56cf2da2cd`
- Classes: **U** = union without interaction; **C** = composed, with one behavior feeding or wrapping the other; **D** = same concept required a controller-governed design choice.

The enumeration below was completed before any conflict was resolved. Hunk names are semantic because line numbers move during resolution.

| # | File | Hunk | Ours intent | Theirs intent | Class | Resolution |
|---:|---|---|---|---|:---:|---|
| 1 | `eval/tier_a/cli.py` | `make_oracle` arguments | Supply lock-recorded LSP initialization options. | Supply bounded query/startup/session budgets and corpus Cargo features. | C | Pass all timeout controls and choose initialization options with lock input taking authority over config-derived Cargo features. |
| 2 | `eval/tier_a/cli.py` | `run_corpus` setup | Accept admitted corpus identity, scrubbed environment, declared inputs, injected authenticated SUT, and deferred metrics. | Verify immutable source manifests, report progress, and construct the ordinary SUT. | D | A supplied lock identity is authoritative and bypasses source-manifest identity; otherwise verify the source manifest. In both modes keep progress and bounded execution, while an injected/authenticated SUT and scrubbed environment remain authoritative. |
| 3 | `eval/tier_a/cli.py` | run metadata identity | Record full lock corpus identity and untracked sources. | Record source-manifest identity, language, and oracle configuration. | D | Emit lock identity fields for lock-bound runs and source-manifest identity for non-lock inputs; retain common language/oracle metadata. |
| 4 | `eval/tier_a/cli.py` | pre-oracle validation | Record scrubbed environment, declared oracle initialization, and SUT boundary state. | Fail closed before oracle launch on immutable-source drift. | C | Record boundary metadata first, then return invalid for manifest drift only on non-lock manifest legs. |
| 5 | `eval/tier_a/cli.py` | oracle construction/accounting | Build the oracle from lock-recorded initialization and retain deferred accounting. | Apply timeout overrides, SUT deadline, per-query failure recording, and progress. | D | Apply main's budgets to both modes; lock initialization remains authoritative for the lock-bound Prism leg; main's config initialization applies elsewhere. Failure accounting feeds the branch's deferred-metrics boundary. |
| 6 | `eval/tier_a/cli.py` | oracle version identity | Compare the observed oracle version to the lock. | Bind tsserver version plus the Node-module environment. | C | Always observe once; enforce lock version when present and tsserver environment/version when selected. |
| 7 | `eval/tier_a/cli.py` | M1 universe/inventory | Use the lock-resolved corpus universe and shared inventory helper. | Restrict immutable inputs to manifest files and retain timed-out-file accounting/progress. | D | Lock universe wins when present; otherwise use the manifest-filtered universe. Both paths record document-symbol failures, and budget exhaustion remains an oracle timeout. |
| 8 | `eval/tier_a/cli.py` | sampling inventory | Use the recorded declared snapshot. | Use live native JS/TS inventory to retain newly supported members. | D | Declared snapshot wins for the lock-bound Prism leg; JS/TS manifest legs use live native inventory; historical snapshots remain the fallback. |
| 9 | `eval/tier_a/cli.py` | final accounting and metrics | Persist probe status and site fingerprints; defer verdict metrics across the SUT boundary when requested; use declared adjudications. | Turn timeout/incomplete/member/pinned errors into invalid reasons and summaries. | C | Main's explicit error invalidations feed either immediate metrics or the branch's deferred boundary state; declared adjudications remain authoritative when supplied. |
| 10 | `eval/tier_a/lsp_client.py` | constructor signature | Accept raw lock initialization options. | Accept bounded session controls and server-status capability. | C | Keep one initialization-options parameter plus all bounded session controls; callers translate lock data into it. |
| 11 | `eval/tier_a/lsp_client.py` | constructor state | Preserve initialization options and the initialize response. | Preserve deadline/dead-process/session state. | C | Store both initialization response and all bounded lifecycle state. |
| 12 | `eval/tier_a/lsp_client.py` | initialize request | Send lock initialization options and retain `initialize_result`. | Send generalized initialization options and retain `serverInfo`. | C | Send the unified options, retain the full result, and derive `serverInfo` from it. |
| 13 | `eval/tier_a/matrix.py` | DFG edge command | Run with the scrubbed SUT environment. | Run through the bounded SUT deadline helper. | C | The bounded helper carries the scrubbed environment. |
| 14 | `eval/tier_a/matrix.py` | DFG stats command | Run with the scrubbed SUT environment. | Run through the bounded SUT deadline helper. | C | The same bounded, environment-preserving helper is used. |
| 15 | `eval/tier_a/oracles.py` | `LspOracle` signature | Accept lock initialization options. | Accept query/startup/session budgets and Cargo feature configuration. | C | Accept explicit initialization options plus budgets and Cargo features. |
| 16 | `eval/tier_a/oracles.py` | LSP client construction | Pass lock initialization options. | Configure bounded lifecycle, Cargo features, and Rust server-status notifications. | C | Explicit lock options win; otherwise derive Cargo-feature options, while all bounded lifecycle controls remain enabled. |
| 17 | `eval/tier_a/pinned.py` | SUT exception row | Preserve the recorded oracle answer and identify the SUT side. | Classify timeout/error outcomes explicitly. | C | Keep the oracle sites and SUT side. Timeouts are explicit; ordinary non-lock errors use main's typed outcomes; lock-bound compatibility errors retain the branch's side-labeled result. |
| 18 | `eval/tier_a/sut.py` | imports | Import `Path` for digest-bound executable resolution. | Import `time` for deadlines. | U | Keep both imports. |
| 19 | `eval/tier_a/sut.py` | constructor/cache policy | Resolve the executable under the scrubbed environment. | Put the navigation cache in a writable workspace path. | D | Preserve executable resolution and environment. Enable main's writable cache only for ordinary non-lock runs; disable it whenever the scrubbed lock environment is supplied because the existing cache key does not prove observed-SUT-digest inclusion. |
| 20 | `eval/tier_a/sut.py` | `_run` invocation | Preserve version handling and pass the scrubbed environment to every subprocess. | Apply bounded SUT deadlines and writable cache arguments. | D | Version and navigation calls are bounded and environment-preserving. Lock-bound instances force `--no-cache`; ordinary instances may use the writable cache. |
| 21 | `eval/corpora.toml` | defaults | Keep existing Rust/Go/Python floors. | Add JS/TS floors and timeout defaults. | U | Keep all floors and timeout defaults. |
| 22 | `eval/corpora.toml` | immutable quick corpora | Use the lock-governed `corpus.prism` identity. | Add a separate manifest-pinned Prism corpus plus immutable JS/TS/Node inputs. | D | Default quick uses lock-governed `prism`; retain immutable `excalidraw-ts` and `secbench-node`; drop the competing `prism-quick` identity. |
| 23 | `eval/pyproject.toml` | script entry point | Keep existing Tier-A/Tier-C scripts. | Add the SecBench runner. | U | Keep all scripts. |
| 24 | `eval/pyproject.toml` | wheel packages | Package existing harness modules. | Package `secbench`. | U | Include all packages. |
| 25 | `eval/tests/test_sut.py` | happy-path subprocess assertion | Assert the scrubbed adapter argv/kwargs. | Assert the default 120-second SUT bound. | C | Assert both the bounded timeout and the adapter's environment behavior in their respective tests. |
| 26 | `eval/tier_a/cli.py` | quick dispatch semantic follow-through | Send admitted Prism quick runs through lock/materialization/declared-input admission. | Select a multi-corpus quick set including immutable JS/TS/Node legs. | D | Default `--quick` remains the single lock-admitted `prism` transaction; explicitly selected JS/TS/Node quick legs run through their immutable source manifests. |

`uv.lock` did not move in this merge, so it has no auto-merged hunk to classify.

Resolution totals: **U 4 / C 13 / D 9**.

## Post-merge folds

The pre-resolution table is retained as the decision record. These later folds
supersede or refine its operational resolutions:

| Commit | Follow-up resolution |
|---|---|
| `7a5f3016` | Superseded rows 19–20: lock-bound legs restored Prism's reusable nav store after `--no-cache` made M2 14x slower and exhausted the budget. |
| `24c75fc9`, `14e2063e` | Restored the Prism readiness/session budgets and cargo-target-only exclusions, then repaired the TOML shape. |
| `289e4c1f` | Eagerly imported `tsserver` before boundary observation so a legitimate harness-module read could not become an undeclared late read. |
| `895bd9a0`, `c29580de`, `03205571`, `48168664` | Fold round 1 hardened oracle-failure partitioning, invalid-leg refusal, scorer-consistent lost-match detection, and identity-shape admission. |
| `0de8fdcb`, `26655b67`, `d405c46c` | Made test tool versions hermetic, invalidated all live-tree measurements, refused lock sample overrides, and refused lockless quick runs. |
| `6d439882`, `8f8b60b9`, `5be1589c` | Bound every lock leg to a private executable snapshot and digest-partitioned cache, isolated harness bytecode before imports, and rebound the committed comparison to committed leg records. |
