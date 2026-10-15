# Handoff — DFG compaction PR A / Task 1

**Written:** 2026-10-15T05:06:42.064428+00:00 · **By:** Codex · **Provider:** codex
**Workspace:** impl-dfgA-clone · perf/dfg-oracle · **Measured state:** `[MEASURED]` HEAD cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e · Tree DIRTY · Probe git status --short / git diff --cached · Output .review/dfg-A/status-final.json
**Predecessor:** none — first in lane
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner assigned Task 1 only to Codex; controller takes publication — RESOLVED.
**(b) Custody exposure** — `[MEASURED]` uncommitted source, patch/message, checkpoint and final evidence archive; implementation Git writes prohibited — OPEN until controller copies receipts and commits.
**(c) In flight / irreversible** — `[MEASURED]` all cargo processes launched by this worker finished; none pending — RESOLVED.
**(d) Authorization granted but not exercised** — "Execute Task 1 (PR A) ... Stop after Task 1." Commit/push remains with controller.

## 1. Resume order

1. Read VERIFICATION.md and docs/superpowers/plans/2026-10-14-dfg-compaction-validation.md, .review/dfg-A/task1-message.txt and .review/dfg-A/owned-files.txt. Do not redo implementation.
2. Copy .review/dfg-A/task1-evidence.tar.gz and .review/dfg-A/task1.patch into durable controller custody before cleanup. Inspect patch and reconstruction receipt.
3. In host context, run missing gates below, then stage the exact 28 owned paths and commit with task1-message.txt. Never git add -A.

**STOP conditions:** free disk below 20 GiB; unexplained pre-A pin/output difference; no replacement baseline, no silent rebase/re-freeze, no Task 2 implementation in this clone.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Producer extraction + frozen oracle | done | `[MEASURED]` 42 indentation-only source controls; 17 frozen blocks unchanged from old plan base |
| Corpus/outputs | done | `[MEASURED]` 154 passed; 1,710 successful paired rows; 109 fixtures/12 total languages |
| Parent payload pin | done | `[MEASURED]` parent and candidate each reproduce all 131 original pins with identical capture adapter |
| Default/MCP subsets | done | `[MEASURED]` 5,421 / 5,937 passed, zero failed; one declared ignored in each; MCP excludes exact 34 socket names |
| Review fix pass | done | `[MEASURED]` no WRONG production finding; two assertion SMELLs fixed with watched update mutant and literal paired controls |
| Release CLI pair | done | `[MEASURED]` parent-bin/prism and candidate-bin/prism plus SHA256 |
| Host gates and commit | pending | `[UNKNOWN]` controller facilities not available to this worker |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Plan source baseline | 1,519 lines / cache v117 | `[MEASURED]` execution parent has 1,576 DFG lines / cache v118; producer-only rebind; nav sidecar stays 65 |
| Future activation title/version | v118 reserved | `[MEASURED]` 118 already occupied; rebind free current-main-plus-one in Task 6 |
| Memory | None | No updates authorized or made |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Host full suites | pending | Run full default and full --features mcp without exclusions | named socket sandbox refusal | socket-exclusions.txt |
| 2 | Tier-A | pending | Fresh release, eval matrix then quick | uv cache .git permission refusal | t1-tier-matrix.log |
| 3 | Mutation gates | pending | Scoped advisory then full authoritative, --jobs 1 and approved cargo wrapper; forecast full catalog time | controller-owned gate | mutants/dfg-compaction.json |
| 4 | Executable replay | pending | Parent/candidate all-generator JSON/review/SARIF/targets + fixture nav dfg-stats --edges, fresh separate caches; only three validated SARIF provenance fields normalized | host replay | parent-bin / candidate-bin |
| 5 | Commit | pending | Inspect exact owned patch, complete host gates, git add -- owned list, git commit -F task1-message.txt | Git write sandbox | task1.patch |

## 5. Invariants and traps — do not do these

- Never write the implementation Git index/commit in this sandbox; no git add -N was used.
- Never bless candidate payloads as parent bytes; preserve the original pins and manifest for B–D.
- Never run concurrent cargo processes, including fmt; nice was refused, owner authorized CARGO_BUILD_JOBS=4. One inadvertent fmt-check overlap was ledgered and corrected.
- Run the feature umbrella sequentially with RUST_MIN_STACK=16777216 and --test-threads=1. Largest exhaustive corpus case is about 59s; registered lib selectors are below one second warm.
- Never enable dfg-oracle in production/release binaries; no compact runtime or performance acceptance exists here.
- Parent/candidate SerializedCpg capture adapter remains test-only; normal production cache source is unchanged.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Parent | `cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e` |
| Saved DFG source SHA256 | `d68e5baec10826c393281909d2c5393c4cdb70497495ab1e310cf7041a6739ac` |
| Ledger | `docs/superpowers/plans/2026-10-14-dfg-compaction-validation.md` |
| Patch / message | `.review/dfg-A/task1.patch` / `.review/dfg-A/task1-message.txt` |
| Evidence | `.review/dfg-A/task1-evidence.tar.gz` and `.review/dfg-compaction/logs/` |
| Cache versions | `118` / navigation sidecar `65`, both unchanged |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "PR A preserves the frozen legacy contract and fixture payload bytes" · pass: INDEPENDENT · evidence tier: TEST-BACKED · record: fresh task1_final_review source review plus author-executed exact source controls, parent/candidate payloads, mutants and full local gates. Reviewer did not independently run tests; host gates remain pending.

**Questions the owner owes an answer to:** None. Controller gates/commit are standing dispatch work.

Verification-hook addendum: `[MEASURED]` root VERIFICATION.md added; previously overstated formatted-return guard now exists and was watched RED/GREEN. All full suites were rerun afterwards. No production code changed. The source/patch manifest now has 28 files.
