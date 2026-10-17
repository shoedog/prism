# Handoff — DFG compaction PR F: oracle retirement, controller review and commits

**Written:** 2026-10-17T20:52:01.807757+00:00 · **By:** Codex root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-dfg-f · test/dfg-retire-oracle · **Measured state:** `[MEASURED]` HEAD 9b2fa09be927ec0f820ba8faf76eb88323b22687 · Tree DIRTY · Probe git rev-parse/status and equivalence/check_source.py · Output /Users/wesleyjinks/prism-evidence/dfg-compaction/prF-2026-10-17/equivalence/F-source-equivalence.json
**Predecessor:** PR E #439, landed 6a63429a. Previous E handoff retained as before-dfg-handoff.md in the evidence directory.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN.
**Provenance:** written live by the worker. `[MEASURED]` claims were rerun this session and point to their outputs. `[INHERITED]` historical acceptance receipts were read/copied, not re-executed. No load-bearing `[ASSUMPTION]`; host-only checks remain `[UNKNOWN]`.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner assigned this lane to Codex; no delegation — RESOLVED for implementation, controller owns review/commits.
**(b) Custody exposure** — `[MEASURED]` 58 uncommitted Git paths (8 added, 22 deleted, 28 modified), plus ignored root VERIFICATION.md, retained in stage snapshots and final-candidate-with-verification/{manifest.json,cumulative.patch,changed-files.tar.gz,SHA256SUMS}. Evidence root: `/Users/wesleyjinks/prism-evidence/dfg-compaction/prF-2026-10-17`. Proposed boundary patches retained — OPEN for controller commits; no worker repository Git writes.
**(c) In flight / irreversible** — `[MEASURED]` full-mutgate.command.json: full authoritative gate exited 0, 549/549 killed/admissible; resources.workers_cleaned=true. All managed cargo/gate sessions completed. No worker command remains in flight — RESOLVED.
**(d) Authorization granted but not exercised** — "PR F changes no production behavior or layout." Controller commits; no publication or merge authority. Do not infer host verification from passing sandbox subsets.

## 1. Resume order

1. Read `REPORT.md`, `VERIFICATION.json`, `files-changed.md` and `proposed-commits/manifest.json` under the evidence root. Worker implementation and runnable gates are complete; no automatic gate retry is needed.
2. On a capable host, run the exact 109 selectors in `mcp-exclusions.txt` with MCP features, recording totals. `[UNKNOWN]` here: Unix socket binds returned EPERM or daemon startup/handshake could not complete; exact names match the inherited E exclusion receipt. Do not retry that denied class in this sandbox.
3. Independently review PR F under its two-round cap. Stage the exact three proposed boundaries (ordinary replacements, production span tie, deletion). The read-only patch checks in proposed-commits/validation.json validate exact before-images. Apply both plan-required trailers when committing.
4. After controller commits, rebind `git diff 9b2fa09b HEAD -- src Cargo.toml Cargo.lock build.rs` to the exact exclusions in equivalence/F-source-equivalence.json. HEAD remains the parent here; its empty literal diff is not certification of the dirty candidate. Current proof is the working-tree exact-hunk audit plus final snapshot.

**STOP conditions:** required production behavior/layout change, unexplained output drift, or failed pin reproduction. Never re-pin. Full authoritative gate was run once at final source and passed. Start no cargo/mutgate/Tier-A after 2026-10-18T07:30:00Z (Sunday 01:30 MDT); stop by then. Review cap two rounds: classify at cap, preserve this artifact, never restart without owner authorization.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Task 7 acceptance / E merge | done | `[INHERITED]` acceptance.md, PR E #439; raw source/probe/binary and corpus/workload bindings copied under equivalence/acceptance-*.json; no new real-corpus measurement |
| Final oracle receipt | done | `[MEASURED]` step0/RESULT.md: pristine 9b2fa09b archive, umbrella 168/0/1, list 169, explicit capture 1/0; both 133-case pin maps reproduce; original/captured manifests and SHA256SUMS |
| Selector inventory and replacements | done | `[MEASURED]` selector-table.md: 169 umbrella selectors, 94 ordinary DFG selectors and external helper callers; infrastructure-crosswalk.md includes 18 retired CI-filter checks. Normal navigation 133 corpus +2 incremental cases; output 21 cases +3 controls; layout invariant retained |
| Span-tie retarget | done | `[MEASURED]` tie-probe/RESULT.md: unmutated 94/0; production byte-sort mutant exact test fails with [10,11) instead of [20,21); other 93 DFG unit tests survive without it; restored exact test 1/0. Survival claim is bounded to that group |
| Before-deletion mutants | done | `[MEASURED]` predelete-mutgate/summary.json: 15/15 killed/admissible, green source and worker baselines, before deleting the engine |
| Oracle deletion | done | `[MEASURED]` deleted-paths.json: 22 files; Cargo feature/target, duplicate engine, oracle modules/converters, umbrella/pins, two CI jobs and path script/test removed. Acceptance tools retained |
| Production source equivalence | done | `[MEASURED]` equivalence/F-source-equivalence.json: exact hunks in Cargo.toml, cpg.rs, data_flow.rs, build/mod.rs, compact/mod.rs and compact/arena.rs; 22 wholly test-only paths. No whole mixed-file exclusion. Production query/storage/wire/producer bodies, Cargo.lock and build.rs unchanged |
| Release output / census | done | `[MEASURED]` release-output-comparison.json: 7/7 byte comparisons after enumerated provenance values only; 9 edge rows and successful three-algorithm/Targets output. physical-storage-comparison.json: 5/5 cases byte-identical, capacities/ranks/IDs included; observer builds distinct from pristine production binaries |
| Final format/default/MCP | done | `[MEASURED]` fmt-final.log; default-postvisibility.log 5,820/0/3 ignored; mcp-final.log and mcp-rayon1.log each 6,409/0/3 ignored, exactly 109 filtered facility checks |
| Release / Tier-A / tools | done | `[MEASURED]` parent/head-release-build logs; matrix 178 ok; quick 3 valid corpora, prism matrix 178 ok, zero regressions/flips. Reports retained; committed baseline unchanged. Acceptance-tool tests 44 passed; CI classifier passed |
| Mutations, advisory and full | done | `[MEASURED]` advisory 408/408 admissible killed; full authoritative 549/549 admissible killed, shared and worker baselines each 370 green, no survivors/timeouts, 3,224.94 seconds |
| Host checks / independent review / commits | pending | `[UNKNOWN]` 109 host-facility tests; controller route, two-round PR F review and three commits; no merge-ready claim |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Previous E handoff | Merge unresolved and old workspace | `[MEASURED]` previous bytes preserved in before-dfg-handoff.md; this is current F custody |
| acceptance.md PR F handoff | Tie mutant anchored temporary engine | `[MEASURED]` now production DataFlowGraph traversal; before-deletion and final mutation proofs retained |
| CLAUDE.md / feature docs | Migration-oracle guidance | `[MEASURED]` normal regression targets and completed retirement documented |
| Tier-A report baseline | Generated run output could overwrite standing reports | `[MEASURED]` original baseline retained and restored byte-identically; generated reports/runs in evidence only |
| Old size/relocation reports | Controls used a sibling source path | `[MEASURED]` final file-sizes.log and relocation-size.json use child path; old reports explicitly renamed before-path-fix |
| attempt1-misread-window | Cutoff treated as Saturday | `[MEASURED]` superseded by owner window ending Sunday 01:30 MDT; old receipt retained |
| Memory | None | No memory update requested or performed |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Host facility verification | pending | Run exact 109 named MCP tests on capable host; record failures without re-baselining | Sandbox socket/daemon facilities | mcp-exclusions.txt, mcp-failure-classification.json |
| 2 | Independent PR F review | pending | Review current 58-path PR artifact and its ignored root verification document, cap two rounds; use selector and production-equivalence ledgers | Controller | REPORT.md, final-candidate-with-verification/ |
| 3 | Commits and HEAD rebind | pending | Commit proposed three boundaries with plan trailers; inspect literal committed HEAD source diff | Controller .git authority | proposed-commits/, equivalence/HEAD-binding.json |

## 5. Invariants and traps — do not do these

- Do not alter production behavior/layout or relabel old acceptance as a new F measurement. Build provenance differs and is explicitly recorded.
- No worker installs, network or repository Git writes. ps inventory and nice setpriority were denied; do not retry those classes. Subsequent cargo used the README-authorized env fallback, jobs 4, serial/offline.
- Physical census excludes BTree node/slack overhead and private RD key heaps equally; it is not process-live-heap measurement. CLI repeated-read equality is not a true-Hit claim; ordinary Rust tests assert CacheResult::Hit.
- An absent Use still propagates to same-line assignments. Incremental retained-file order persists across both edit/deletion. Same-environment base controls established test expectation/placement errors before test-only corrections; production was not repaired.
- Do not treat the empty current parent-to-HEAD diff as dirty-tree certification or worker self-checks as independent review.
- New files are under 600 weighted lines. Existing tests/common/mod.rs remains 1,225.5 (parent 1,223.5) with only two module declarations; helpers are separate. file-size-exception.md records the reason. No changed file is over 2,000; no oversized production file grows. Conservative new/rewritten Rust surface 851 lines, moves separate.

## 6. Identifiers

| Item | Verbatim / evidence |
|---|---|
| F parent / unchanged Git HEAD | `9b2fa09be927ec0f820ba8faf76eb88323b22687`; working candidate in final-candidate-with-verification/ |
| Measured activation / exact parent | `3739ab383a6e333e3c172bcb1eaf26fcf3d26b92` / `efbe63b702a835748d9eebd19805a4a2ba083f9e` |
| Measured probe ports | `[INHERITED]` dirty ports on those same source bases; patch hashes base `1f98494c30163fd6f97c8794125f2d78c7813eff567396bedb742ef770699d17`, candidate `ab76df03bf72043072fadee9c8073c304a859756d482527331402b15b33d25a9`; original reference c3162f94 |
| Landed E #439 / exact parent | `6a63429abb07aef4da7ca3e152e79a9a8897762a` / `d337ce9711971c36e6566b1d7e8727b37cf4a498` |
| Historical acceptance hashes | `[INHERITED]` equivalence/acceptance-source-probe-hash-index.json, acceptance-binary-receipts.json, acceptance-set-1-before-inventory.json; source/archive, binary, corpus commit and workload/MANIFEST SHA256s retained; no claim of live corpus re-verification |
| Final parent/head production build IDs | `f6922ebfacdf276faf6d59b71f62d6efcb60094de4aa3299009fe8d875dc8bf0` / `18f467445ca8c0e9586993bdaf739c7577359ba6e01190b4ae4fb2571d0b3667`; binaries and SHA256s in equivalence/release-{parent,head}/build-receipt.json |
| Parent/head census observer IDs | `025ba5bb016322b530cf8e34921ea20e77a371232b1a56ce3d7230521b24c4a5` / `18f467445ca8c0e9586993bdaf739c7577359ba6e01190b4ae4fb2571d0b3667` |
| Durable evidence | `/Users/wesleyjinks/prism-evidence/dfg-compaction/prF-2026-10-17` |
| Oracle archive / suite target | `/private/tmp/prism-dfg-f-20261017-final-oracle/main` / sibling `target` |
| Mutgate target | `/private/tmp/prism-dfg-f-20261017-mutgate-target`; isolated workers cleaned |
| Current versions / local advisory main | CPG 122, navigation sidecar 67; advisory main `cb0eeec062053f3b7d29b5d5c5eda8efa30e69da` unchanged during run |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "F changes only test-reachable code, oracle feature/target declarations, CI and docs; production behavior/layout is unchanged" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: equivalence/F-source-equivalence.json, release-output-comparison.json, physical-storage-comparison.json, full-mutgate/summary.json; independent review remains controller work.
**Questions the owner owes an answer to:** None for worker implementation. **SMELL MATERIAL — verification limitation:** the 109 exact host-facility checks remain open; controller must run them on a capable host. No unsettled demonstrated production WRONG is asserted. Commits, final HEAD binding, independent review and merge remain separate authority.

Hook closure: root VERIFICATION.md added after the passing gates; exact commands/totals and all 109 environment-limited exclusions are recorded there. The only candidate changes since the tested source are Markdown documents. Earlier final-candidate/ snapshot is preserved as the 58-path pre-hook state.

Root document custody: existing .git/info/exclude ignores /VERIFICATION.md. The file is included explicitly in final-candidate-with-verification/ (59 paths total); proposed commit patches retain the 58-path PR scope. No .git policy or force staging was performed. root-verification-ignore-provenance.txt records the rule.
