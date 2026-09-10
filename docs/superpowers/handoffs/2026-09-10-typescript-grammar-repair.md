# Handoff — TypeScript grammar repair, awaiting dependency decision

**Written:** 2026-09-10 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · fix/typescript-import-type-generic-grammar
**Measured state:** `[MEASURED]` base9270ab43, tests/docs only; git diff of
src/build.rs/Cargo.toml/Cargo.lock is empty. This is WIP, not merge ready.
**Predecessor:** PR304 merged9270ab43, fetched this turn.
**Truth ordering:** measured live state > explicit owner authority within scope > this handoff > history.
**Provenance:** written live; `[MEASURED]` results below are fresh. PR304 full gates
are `[INHERITED]`, not verification of this incomplete branch.

## 0. Gating facts — settle these before starting anything below

(a) Lane ownership RESOLVED: primary design/tests; grammar_candidate_audit and
grammar_consumer_audit completed read-only investigations. No delegated writer.
(b) Custody RESOLVED: WIP tests/spec committed566a514e and pushed to the named
remote branch. Evidence archive verified below. No PR opened; do not merge RED tests.
(c) In flight: no implementation/generator/install. Dependency authority OPEN:
owner was asked to approve pinned vendoring and isolated generator bootstrap.
(d) Authorization: “merged -> proceed to next”; bounded grammar repair. Vendoring
expansion has not been approved at this checkpoint; do not infer it from this handoff.

## 1. Resume order

1. Run `git status --short --branch`; read adjacent repair plan.
2. Read owner response to the vendoring/bootstrap question. If absent, stop before
   acquisition, install, dependency mutation or generated-code vendoring.
3. If approved, retain these RED tests and follow dependency/cache prerequisite
   sequence before grammar candidate and runtime-call guard. Review cap two rounds.

STOP on missing authority, unreproduced generator baseline, open-class repair scope
or attempted source rewriting/error suppression. Do not merge this WIP branch.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Baseline | done | `[MEASURED]` PR304 merged9270ab43; clean starting checkout |
| Upstream search | done | issue367 open; no verified fix; two generated C files17,515,764 bytes |
| Structural RED | done | structural-red.log:6 passed/12 failed;50 existing tests filtered |
| Existing TS control | done | typescript-control.log:50 passed;18 new tests filtered |
| Consumer proof | done | consumer-fresh.log from freshly rebuilt unchanged library |
| Packaging authority | blocked | awaiting owner vendoring/bootstrap decision |
| Production repair | next | none implemented; original grammar and runtime leaks remain |
| Full gates/review/PR | next | no implementation candidate; no green or merge-ready claim |

## 3. Corrections to standing documents and memory

PR304 merged. Its zero-error controls did not claim tree/call correctness; fresh
structure tests now reveal await get<T>() association/callee defects. Type-only
runtime calls are also proven wrong independently of grammar parse errors.
No memory update authorized or made. Predecessor documents link this WIP handoff.

## 4. Open work

Owner decision on ~17 MiB generated dependency plus isolated pinned bootstrap.
After approval: reproducible vendoring and cache identity, grammar structure repair,
runtime-call exclusion, full gates and Tier-A. Keep scope bounded as adjacent plan.

## 5. Invariants and traps — do not do these

- Never merge or mark ignored the12 failing structural cases to manufacture green.
- No production/parser/dependency modification has happened yet; preserve that claim.
- Do not conflate parse-error counts with correct await/call structure.
- Do not flatten any await callee unconditionally or suppress all import/typeof calls.
- Do not omit vendored bytes from build/cache identity or rely only on a version bump.
- No real corpus edits/installs/execution, private reads, JSON/.mts admission or closure expansion.

## 6. Identifiers

Evidence `/private/tmp/prism-grammar-repair-BR8ivP`.
Branch `fix/typescript-import-type-generic-grammar`.
Regression module `tests/lang/typescript/import_type_grammar_test.rs`.
Upstream issue `https://github.com/tree-sitter/tree-sitter-typescript/issues/367`.
Published grammar source `f975a621f4e7f532fe322e13c4f79495e0a7b2e7` (0.23.2).
Pinned compiler `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js`.
Prior classification evidence `/private/tmp/prism-parser-classification-6vOV0e`.
Archive `/private/tmp/prism-grammar-repair-BR8ivP-evidence.tgz`, mode0600,
12,354,257 bytes; SHA256
`5773d57c1724d3933abfb840e59d6316bd44cd94d0823acee629b3e7f89eb5a9`.
Gzip integrity verified; immutable WIP diagnostic checkpoint, not a completed repair.

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — no repair candidate exists; pass: SELF-PASS (NOT INDEPENDENT)
for WIP test/spec custody only; evidence tier: TEST-BACKED diagnostics. Independent
upstream/consumer audits informed design but are not implementation approval.

**Questions the owner owes an answer to:** Approve pinned in-repo vendoring and an
isolated pinned generator bootstrap, with generated output reviewed separately?
