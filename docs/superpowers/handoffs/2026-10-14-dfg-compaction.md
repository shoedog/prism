# Handoff — DFG compaction PR A / Fix 2

**Written:** 2026-10-15T06:32:36.791562+00:00 · **By:** Codex · **Provider:** codex
**Workspace:** impl-dfgA-clone · perf/dfg-oracle · **Measured state:** `[MEASURED]` HEAD cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e · Tree DIRTY · Probe git status --short, source copy and pin audit · Output .review/dfg-A/fix2/baseline-source-manifest.json, provenance-audit.json
**Predecessor:** Fix1 handoff; exact copy retained in .review/dfg-A/fix2/baseline/docs/superpowers/handoffs/2026-10-14-dfg-compaction.md.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` user assigned this bounded Fix2; controller owns publication — RESOLVED.
**(b) Custody exposure** — `[MEASURED]` pre-fix source snapshot and original artifacts retained; uncommitted Fix2 — OPEN until controller copies/commits.
**(c) In flight / irreversible** — `[MEASURED]` all worker checks complete, no pending worker process; packaging/application receipts retained — RESOLVED.
**(d) Authorization granted but not exercised** — "Write .review/dfg-A/fix2.patch (incremental on fix1)" completed; Git writes remain controller work.

## 1. Resume order

1. Read `.review/dfg-A/fix2/patch-verification.json` and `full-suite-results.json`; copy `fix2.patch`, `fix2-message.txt`, `fix2-evidence.tar.gz`, and `fix2-artifacts.sha256` into controller custody.
2. Apply `fix2.patch` over exact Task1 plus Fix1 (not local pre-Task1 HEAD). Stage its five exact paths, excluding VERIFICATION.md; commit with `fix2-message.txt` under controller authority.
3. Complete prior host-only duties: 34 excluded socket tests, fresh release then Tier-A matrix/quick, committed-tree scoped/full authoritative mutation and executable replay. Fix2 itself changes no production code.

**STOP conditions:** pin/source mismatch; disk under 20 GiB; two-round cap exceeded; no Task2 or recapture.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Pin provenance audit | done | `[MEASURED]` provenance-audit.json: 131 verified original hashes/lengths and parent bytes |
| Same-source retained-pin control | done | `[MEASURED]` before-with-pins.json: named Python 1/0 |
| No-review umbrella RED/GREEN | done | `[MEASURED]` before-no-review.json: 24/130; after-no-review.json: 159/0, env unset |
| Manifest negative coverage | done | `[MEASURED]` negative-controls.json: corrupt/missing/unused rejected, restored controls 1/0 each |
| Full suites | done | `[MEASURED]` full-suite-results.json: default 5421/0/1; MCP+oracle 6096/0/1; 34 socket exclusions |
| Incremental patch | done | `[MEASURED]` patch-verification.json; five exact paths, no report/payload bytes |
| Fix1 gates | done | `[INHERITED]` fix1/after-results.json and mutation-result.json; no new mutation claim |
| Publication | pending | `[UNKNOWN]` controller facilities/authority unavailable here |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Fix1 oracle GREEN | Portable checkout proof | `[MEASURED]` depended on clone-local pins; Fix2 no-review umbrella 159/0 |
| Validation ledger | Retained bytes sufficient custody for tests | `[MEASURED]` Fix2 section records committed digest manifest requirement and audit |
| VERIFICATION.md | Fix1 report | `[MEASURED]` Fix2 results prepended; prior reports labeled historical; excluded from patch |
| Immutable Task1/Fix1 receipts | Prior evidence | `[INHERITED]` preserved; never rewritten as new results |
| Memory | None | No updates authorized or made |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Fix2 publication | pending | Copy/apply/commit exact five paths | Controller authority | fix2.patch, fix2-message.txt |
| 2 | Host-native gates | pending | Prior Fix1 handoff host duties | Controller facilities | socket-exclusions.txt; original receipts |
| 3 | Commit/custody | pending | Copy/apply/commit exact Fix2 paths over Fix1 | Controller authority | fix2.patch, fix2-message.txt |

## 5. Invariants and traps — do not do these

- Preserve original pins and immutable Task1/Fix1 artifacts; hash the pre-A capture, never the candidate.
- No Git/index writes, socket binds, uv or network here; one Cargo process at a time, jobs=4 under inherited refused-nice override.
- Use offline Cargo with RUST_MIN_STACK removed. Zero selection or compilation failure is no behavioral control.
- Fix2 changes only tests, manifest, ledger and this handoff; VERIFICATION.md stays outside its patch.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Local/pre-A parent | cc2e612c04d4ab84a7b6843b8c3c16e1ad2ffc5e |
| Digest manifest | tests/dfg_compaction/pins.sha256 |
| Baseline/copy | .review/dfg-A/fix2/baseline / clean-tree |
| Source snapshot | .review/dfg-A/fix2/pre-fix2-source.tar.gz |
| Ledger | docs/superpowers/plans/2026-10-14-dfg-compaction-validation.md |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "The oracle carries all pre-A digest pins and runs without .review" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: .review/dfg-A/fix2/before-no-review.json, after-no-review.json, negative-controls.json, full-suite-results.json, patch-verification.json

**Questions the owner owes an answer to:** None.
