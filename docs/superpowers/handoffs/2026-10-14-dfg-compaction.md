# Handoff — DFG compaction PR A / Fix 1

**Written:** 2026-10-15T05:41:26.483735+00:00 · **By:** Codex · **Provider:** codex
**Workspace:** impl-dfgA-clone · perf/dfg-oracle · **Measured state:** `[MEASURED]` HEAD cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e · Tree DIRTY · Probe git status --short / git diff --cached / Task1 reconstruction · Output .review/dfg-A/fix1/final-status.txt, task1-reconstruction.log
**Predecessor:** Task1 handoff, refreshed in place for controller Fix1; original evidence preserved.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` user assigned both controller gate repairs to this worker; controller owns publication — RESOLVED.
**(b) Custody exposure** — `[MEASURED]` uncommitted fix, patch/message and evidence archive under .review/dfg-A; no Git write authority — OPEN until controller copies/commits.
**(c) In flight / irreversible** — `[MEASURED]` all worker Cargo/mutgate checks completed; none pending — RESOLVED.
**(d) Authorization granted but not exercised** — "Fix both in this working tree, test-first where it applies" completed; no commit/push authority exercised.

## 1. Resume order

1. Read VERIFICATION.md (current Fix1 report first) and .review/dfg-A/fix1/patch-verification.json. Copy fix1.patch, fix1-message.txt and fix1-evidence.tar.gz to controller custody.
2. Apply fix1.patch over exact Task1 tree (controller-supplied commit 9be9ffb6 elsewhere); never apply it directly over this local pre-Task1 HEAD. Review exact seven changed paths.
3. Run native scoped planner/gate after Task1 files are tracked; complete remaining host-only gates. Commit exact paths with fix1-message.txt under controller authority.

**STOP conditions:** free disk below 20 GiB; unexplained source/pin mismatch; two-round cap exceeded; no Task2, re-freeze or mutant weakening.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Task1 identity | done | `[MEASURED]` all 28 owned files matched reconstructed task1.patch before repair |
| 19 mutant relocations | done | `[MEASURED]` relocations.json/md, scope-audit.json; tokens/selectors intact |
| Default-stack oracle | done | `[MEASURED]` before abort, unchanged-source large-stack 1/0 control; fixed named 1/0 and umbrella 154/0 with env unset |
| Default / runnable MCP | done | `[MEASURED]` 5,421/0/1 and 5,937/0/1; MCP exact 34 socket exclusions |
| Formatting | done | `[MEASURED]` after-fmt.log; git diff --check |
| Full planner / scoped execution | done | `[MEASURED]` 425 valid anchors; overlay scoped 19/19 admissible KILLED, advisory |
| Incremental packaging | done | `[MEASURED]` fix1/patch-verification.json; patch and evidence SHA256 receipt |
| Host-native gates / publication | pending | `[UNKNOWN]` controller facilities/authority unavailable here |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Original stack workaround | Current tests require RUST_MIN_STACK=16777216 | `[MEASURED]` replaced by production pool wrappers; all requested suites passed with variable unset |
| Validation ledger | Old workaround sufficient | `[MEASURED]` historical ruling marked superseded; two repairs and scope overlay ruled explicitly |
| VERIFICATION.md | Task1 gate status only | `[MEASURED]` current Fix1 report prepended; original Task1 commands labeled historical |
| Immutable Task1 artifacts | Earlier receipt snapshots | `[INHERITED]` preserved as historical evidence; new incremental artifacts carry current state |
| Memory | None | No updates authorized or made |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Native scoped run | pending | After tracked Task1+Fix1: python3 scripts/mutgate/mutgate.py --since main --scope fn --plan-only, then normal scoped gate | Local untracked destinations / no Git writes | scoped-live-tree-population.json |
| 2 | Full authoritative mutation execution | pending | Run full catalog on host, jobs=1 and approved build slot | Controller schedule | 425 mutants |
| 3 | Socket/Tier-A/CLI replay host gates | pending | Original Task1 host duties; release then Tier-A; exact excluded sockets and executable replay | No binds/uv/network here | socket-exclusions.txt; original task1 evidence |
| 4 | Commit and durable custody | pending | Copy receipts, inspect/apply fix1.patch, commit exact paths | Controller authority | fix1-message.txt |

## 5. Invariants and traps — do not do these

- No implementation Git writes, socket binds, uv or network in this worker environment. One Cargo process at a time, jobs=4 under refused-nice override.
- Run current oracle without RUST_MIN_STACK; test bodies use production 256 MiB pool.
- Native scope selects zero in this untracked clone; zero selection is no coverage. Overlay adds only saved Task1 src additions to the native changed-line set and leaves the driver/index unchanged.
- Advisory 19/19 KILLED is not the full authoritative mutation gate.
- Preserve all original pre-A pins and task1.patch/message/evidence; fix1 is incremental.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Local HEAD | cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e |
| Controller Task1 commit | `[INHERITED]` 9be9ffb6 elsewhere |
| Patch / message | .review/dfg-A/fix1.patch / .review/dfg-A/fix1-message.txt |
| Evidence archive / hashes | .review/dfg-A/fix1-evidence.tar.gz / fix1-artifacts.sha256 |
| Results | .review/dfg-A/fix1/after-results.json / mutation-result.json |
| Ledger | docs/superpowers/plans/2026-10-14-dfg-compaction-validation.md |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "Fix1 restores all moved anchors and runs every oracle case without an environment stack override" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: .review/dfg-A/fix1/before-results.json, after-results.json, scope-audit.json, test-body-preservation.json, mutation-result.json; native untracked-scope limitation explicit

**Questions the owner owes an answer to:** None.
