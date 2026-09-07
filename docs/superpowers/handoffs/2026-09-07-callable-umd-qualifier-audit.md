# Handoff — UMD qualifier source audit

**Written:** 2026-09-07 · **By:** /root · **Provider:** codex
**Workspace:** /private/tmp/prism-qualifier-audit-5LdjYI/worktree · audit/callable-umd-qualifier · **Measured state:** `[MEASURED]` base dc9b9e84; audit/tests captured in d044a2b; verification complete; production observer unchanged.
**Predecessor:** PR267–269, confirmed merged through dc9b9e847b829bf84a43a323a9eb7e28e6aa5c16.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) /root only; original dirty checkout preserved using isolated worktree — RESOLVED.
(b) Audit/tests d044a2b and verification/evidence70fc728 pushed;
    https://github.com/shoedog/prism/pull/270 open against main — RESOLVED publication;
    remote CI/review remains a separate gate, not a local test claim.
(c) Observer124/124; Rust default4017/MCP4207 passed,0 failed,1 ignored each,
    including doctests; authority verifier clean and4/4 controls passed — RESOLVED.
(d) Owner: “267, 268, 269 are merged, proceed to next”. Next was a source/compiler
audit and negative fixtures, not automatic UMD resolution expansion.

## 1. Resume order

1. `git -C /private/tmp/prism-qualifier-audit-5LdjYI/worktree status --short --branch`.
2. Verify remote publication and PR checks; full gate logs and compact committed
   evidence are recorded in the readout. Do not rerun acquisition or install.
3. Proposed implementation requirements are in callable-umd-qualifier-proof spec;
   do not infer runtime or broad global/merged-namespace support from the audit.

**STOP conditions:** two self-review rounds; no new installs, private app writes,
runtime expansion, React spelling heuristics or removal of duplicate/closure barriers.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Merges/isolation | done | `[MEASURED]` PR26733cc2e47,268ec0b29db,269dc9b9e84; original dirty state preserved |
| Actual qualifier population | done | `[MEASURED]` observer-symbols.json and direct-compiler.json: raw singleton UMD alias/export=/local namespace |
| Production instrumentation | done | `[MEASURED]` removed; packet comparison differs only in producer hash; normal main validates predecessor packet |
| Characterization fixtures | done | `[MEASURED]`15 new tests; full observer124 passed,0 failed,0 skipped |
| Public source custody | done | `[MEASURED]` before/after manifests equal, original clean; no install or source edits |
| Full project gates | done | `[MEASURED]` default4017/MCP4207 passed,0 failed,1 ignored each; authority failures=[] and4 tests passed; fmt/diff clean |
| UMD resolution implementation | next | separately bounded proposal; no implementation in this audit |

## 3. Corrections to standing documents and memory

Prior stack/CI-pending statements are historical; all three PRs are merged.
Qualifier ambiguous_declaration is not evidence of actual React augmentation here:
the observer mixes global-export and module-local name domains. Normal unproven
outcome remains correct for the currently unsupported UMD gateway. No whole-Program
augmentation closure or runtime WRONG established. No memory edits authorized.

## 4. Open work

PR270 is published against main; all local gates and two self-review rounds are
complete. Check current remote CI/review before merging; no merge performed here.
Evidence archive: task-root `qualifier-audit-evidence.tgz`, SHA256
`ea201d05d083e2ee69e73dcd9ae57349155f0f1c138b433c8d260a1bca729abb`.
Compact evidence, source anchors and executable fixtures are committed; the full
archive remains local and is not required to execute the fixtures.
Next proposal: bounded generic UMD declaration bridge with source-identity and
global provider/duplicate checks, not React.FC spelling expansion.

## 5. Invariants and traps — do not do these

- A recovered symbol can have zero declarations; require source evidence.
- A unique raw alias does not erase competing provider or augmentation populations.
- allowUmdGlobalAccess type-use and value-use questions are different.
- Diagnostic instrumentation must not ship; producer hash is unchanged on this lane.
- LSP tools unavailable; exact compiler/source fallback used, not structural Prism inference.
- Native compiler fixture roots must be canonical on macOS (/private/var versus /var).

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task root | `/private/tmp/prism-qualifier-audit-5LdjYI` |
| Public acquired source | `/private/tmp/prism-acquire-w2FtSq/source` |
| Compiler | `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js` |
| Profiles | `/private/tmp/prism-callable-authority-98TLLN/public/profiles` |
| Base | `dc9b9e847b829bf84a43a323a9eb7e28e6aa5c16` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "the reported qualifier ambiguity is scope-mixed candidate evidence, not multiple compiler declarations at this hop" · pass: two SELF-PASS rounds (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: observer-symbols.json, observer-full.log, proof spec; full project gates complete

**Questions the owner owes an answer to:** None for this audit; the successor bridge is a proposed bounded implementation.
