# Handoff — bounded TypeScript erased-rvalue traversal

**Written:** 2026-09-10 · **By:** root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · fix/typescript-erased-rvalue-traversal · **Measured state:** `[MEASURED]` HEAD 70caaf6d43f9bbd1ecf47279af0db05ce02753f4 · Tree DIRTY · Probe `git status --short --branch` · Output implementation, tests, plan and this handoff uncommitted.
**Predecessor:** PR305; 2026-09-10-typescript-grammar-repair.md.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` root owns design/implementation; classification_review supplied read-only source/compiler verification — **RESOLVED**.
**(b) Custody exposure** — `[MEASURED]` implementation and new tests snapshotted as pre-gates-source.tgz (mode0600); matrix passed, checkpoint commit follows — **RESOLVED locally**, remote custody pending.
**(c) In flight / irreversible** — `[MEASURED]` focused tests and matrix completed; MCP release build running, full gates follow — **RESOLVED**, no overlapping source edits.
**(d) Authorization granted but not exercised** — owner: “merged, proceed to next”; standing “commit and push and open pr”. No automatic merge.

## 1. Resume order

1. `git status --short --branch` in the workspace; reread this handoff, preserve unrelated state.
2. Build release and run Tier-A matrix before commit; quick before review. Run full default/MCP/owner-audit Rust, observers/helpers/authority, Python, example, formatting and Clippy gates.
3. Independent review, at most two formal rounds; close bounded findings. Commit/push/open PR after verification; report invalid/excluded gates truthfully.

**STOP conditions:** new authority requirements, open-class review findings at cap, or unrelated failing gates without same-environment attribution control. Never rebaseline.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Baseline | done | `[MEASURED]` PR305 merge70caaf6d; existing TS suite 88 passed, base-typescript.log |
| RED | done | `[MEASURED]` complete-red-all.log: 2 private failures, 9 integration failures, 1 runtime positive passed |
| Implementation | done | `[MEASURED]` shared ancestry entry/local boundary pruning; semantic-use ancestry refusal |
| Focused GREEN | done | `[MEASURED]` focused-green.log: 2 private +10 integration passed |
| Compiler fixture validity | done | `[MEASURED]` compiler/receipt.json: 55 positive checks, 1 expected TSX angle-assertion rejection, 0 unexpected failures/skips; emitted JS inspected, never executed |
| Tier-A matrix | done | `[MEASURED]` fresh release build, tier-a-matrix.log: 159 cases passed |
| Full gates/review/publication | next | `[UNKNOWN]` not yet run on this implementation |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Prior PR305 handoff | published, merge pending | `[MEASURED]` merged at70caaf6d; this successor governs current lane |
| Initial RED control | const object declaration expected to emit rvalues | `[MEASURED]` existing API does not classify variable_declarator as assignment; corrected fixture to assignment, no production declaration expansion |
| Initial probe logs | all failures are behavioral evidence | `[MEASURED]` bad control, invalid cargo filter and private helper compile error are inadmissible; complete-red-all.log is the valid complete RED |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Full gates and review | next | Resume order above | None | evidence root below |
| 2 | Compound member path containing an asserted receiver | parked | Separate bounded source/native proof before repair | Separate slice | `(runtime as TYPE).X` whole-member text serialization |

## 5. Invariants and traps — do not do these

- No grammar/vendor/bootstrap, receiver/closure admission, React.FC or unresolved react-scripts policy changes.
- No claim of comprehensive TypeScript erasure or compound path normalization; enclosing member text remains unchanged.
- Preserve runtime imports/options/labels, raw source/tree, UTF-8 spans and duplicate/write/epoch barriers.
- No source execution, dependency install, private source reads, full multi-corpus or live-model runs.
- Escalate network, .git writes and uv host cache upfront; do not use IPC-blocked npx tsx.
- A stale Prism index supplied navigation hints only; current source confirmed consumers, not LSP-exhaustive references.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base | `70caaf6d43f9bbd1ecf47279af0db05ce02753f4` |
| Branch | `fix/typescript-erased-rvalue-traversal` |
| Evidence root | `/private/tmp/prism-erased-rvalue-avuSbL` |
| Compiler | `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js` (5.9.3) |
| Compiler SHA256 | `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` |
| Plan | `docs/superpowers/plans/2026-09-10-typescript-erased-rvalues.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — formal review pending · claim: "supported erased type subtrees do not emit runtime rvalues while runtime controls survive" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: focused-green.log; independent review pending.

**Questions the owner owes an answer to:** None.
