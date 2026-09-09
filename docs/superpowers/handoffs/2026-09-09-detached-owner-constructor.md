# Handoff — detached executable-owner constructor

**Written:** 2026-09-09 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/detached-executable-owner-proof · **Measured state:** `[MEASURED]` all seven full gates passed on clean d0a1ce9; additional full MCP+audit4221 passed. A test-helper-only lint simplification is now being reverified before publication.
**Predecessor:** PR295 design and characterization.
**Truth ordering:** measured state > explicit owner authority within scope > this handoff > historical snapshots.
**Provenance:** written live; source and tool checks this turn.

## 0. Gating facts — settle these before starting anything below

(a) Ownership RESOLVED: primary owns design, implementation and verification; no delegates.
(b) Custody: feature branch preserves previous design branch; evidence `/private/tmp/prism-detached-owner-5hHiNu`.
(c) In flight: detached implementation only; no production consumer or cache writes.
(d) Authority: owner approved implementing direct Props/property/class provenance, matching Prism/Program inputs, epoch-bound executable ownership, per-predicate negatives and cross-epoch substitution tests. Closure/react-scripts decisions unchanged.

## 1. Resume order

1. Check git status and HEAD; read approved PR295 spec and current constructor plan.
2. Constructor and two bounded self-review rounds complete; do not restart implementation.
3. Full suites and 54-case exact-base observer parity passed. Finish final test-helper revalidation and publication with exact evidence and exclusions recorded.

STOP on production wiring, closure waiver, install requirement, or open-class findings at review cap.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Base/navigation | done | fetched4e88d33b; Prism method-slot callers stale, LSP unavailable; checked source directly |
| RED/constructor | implemented | scaffold-red.log: 1 actual failure; constructor-final-focused.log: 13 passed; direct-predicates-green.log: 103 passed |
| Input-census defect | fixed | omitted-index-red.log fails before actual loader census; constructor-index-green.log passes after; subset cannot supply full census |
| Full gates | passed on d0a1ce9 | observer694, default4025, MCP4215, MCP+audit4221, helpers18, authority40; one ignored per Rust run; doctests/fmt/diff pass |
| Final helper/publication | pending | test-only type-complexity SMELL simplified; rerun full Rust suites, no production change |

## 3. Corrections to standing documents and memory

PR295 is merged and this slice implements its detached constructor. Production integration is not approved by this slice. No memory edits. The observer factory refactor changes producer bytes but not schema20/version0.21.0 fields or closure semantics; exact fingerprint binds the refactor. Original scaffold output is retained, but its overwritten source was not separately archived; do not claim otherwise. Refusal controls are not behavioral RED.

## 4. Open work

| Work | State | Next action |
|---|---|---|
| Detached constructor | implemented | same-Program facts, actual full Prism census, privately owned/reparsed inputs, 21 mapped anchors, Arc-bound proof |
| Verification | full gates green on d0a1ce9 | final helper revalidation and receipts next |
| Publication | pending | commit/push/PR after gates |

## 5. Invariants and traps — do not do these

- No runtime resolver/CLI wiring, cache migration, class guesses in pre_resolved_target or React.FC widening.
- Packet JSON is not an authority object; only privately acquired/reproduced evidence may mint one.
- Epoch owns immutable parser inputs; never accept a caller-provided subset as the full census.
- Standard compiler libraries are a separate non-executable domain; project declaration/augmentation effects remain fenced.
- Keep default Rust free of a TypeScript installation requirement; compiler-backed audit is an explicit test feature/gate, not a silent skipped test.

## 6. Identifiers

Base4e88d33bc02ceb576fe5ab1b936d3441b3565274. Evidence `/private/tmp/prism-detached-owner-5hHiNu`. Pinned compiler `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js`, SHA3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675.

## 7. Refutation verdict and owner questions

**§2c verdict:** BOUNDED PASS — two self-review rounds completed. Round 1 WRONG: caller-subset map could omit an extra indexed source; captured exact failure and fixed with real loader census between snapshots. Round 2: no additional demonstrated wrong result. Evidence tier: real compiler, parser and epoch substitution tests; not independent-agent review. Immutable historical epoch readout is intentional; no active-analysis manager or persisted-edge lifecycle is implemented.

**Questions the owner owes an answer to:** None within the approved boundary.
