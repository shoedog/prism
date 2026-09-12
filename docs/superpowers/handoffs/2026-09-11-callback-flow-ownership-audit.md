# Handoff — callback flow ownership classification

**Written:** 2026-09-11 · **By:** root · **Provider:** codex
**Closeout:** 2026-09-12; seventh local bundle commit, parent7bdf9981, message
`test(js-ts): classify callback argument-flow ownership gaps`. No publication.
**Workspace:** /Users/wesleyjinks/code/slicing · feat/js-ts-module-binding-audit · **Measured state:** `[MEASURED]` production7bdf9981 plus frozen test-only audit module/registration and docs; baseline receipt verifies registration-stripped build.rs equals base and pins hashes.
**Predecessor:** sixth increment7bdf9981, local/unpushed.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live; /private/tmp/prism-namespace-flow-MhHdDf.

## 0. Gating facts — settle these before starting anything below

(a) RESOLVED: root test/design owner; existing read-only reviewer. Two-round cap.
(b) RESOLVED: committed base7bdf9981; source-checkpoint.tgz preserves the initial
stable audit. The committed baseline receipt pins the final frozen source/tests.
(c) RESOLVED: local verification only; no publication or production repair.
(d) Owner: "proceed to next, will bundle in same MR". Approved next classification
and proof requirements; do not silently turn it into production admission.

## 1. Resume order

1. `git status --short --branch`; read adjacent callback-flow-ownership plan.
2. Source/tests frozen:3audit tests pass; explicit desired-behavior test remains RED.
3. Verification complete with exclusions below; round2 findings closed. Await owner
   choice of bounded repair or publication of the seven-increment local bundle.

STOP: expanded occurrence/index policy, authority broadening, open-class findings,
missing fixtures represented as green, untracked unrelated changes.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Initial15-row probe | done | [MEASURED] initial-probe.log:6missing callback edges,9positive controls |
| Namespace hypothesis | done | [MEASURED] falsified; bare and namespace callbacks share failure |
| Ownership mechanism | done | [MEASURED] src/ast.rs rvalue query/RHS walk; DFG retains real Use; CPG selects parameter token |
| Round1 | done | [MEASURED] reviewer confirms2WRONG defects and2scope/index concerns for future repair |
| Scope correction | done | [MEASURED] simple const function/arrow/object initializers flow; assignment-kind gate excludes variable_declarator |
| Audit | done | [MEASURED] classification-verified.log3pass/0fail/1known-gap ignore;42full +27epoch +6nested rows |
| Desired behavior RED | done | [MEASURED] desired-behavior-red.log0pass/1fail,6missing edges out of15rows |
| Round2 | done | [MEASURED] FIX W2/S0: desired-edge endpoint/confidence and initializer path refusal assertions |
| Targeted review closure | done | [MEASURED] root reused strict observe in ignored regression and asserted both path dispositions; refreshed focusedGREEN/desiredRED |
| Helpers | done | [MEASURED] final binaries:Node786/authority40/Python940pass; Python1live skip, Node3historical fixture tests excluded |
| Tier-A | done | [MEASURED] post-review fresh matrix159pass; quick300024ms timeout/noverdict, not passed |
| Default Rust | done | [MEASURED] final full-default.log4495pass/0fail/2ignore |
| MCP Rust | done | [MEASURED] final full-mcp.log4688pass/0fail/2ignore |
| Audit Rust | done | [MEASURED] final full-audit.log4711pass/0fail/2ignore |
| Examples | done | [MEASURED] examples.log32pass/0fail/0ignore |
| Formatting/Clippy | done | [MEASURED] fmt/check; post-review-clippy.log completed,257warning lines, none in new audit file |

## 3. Corrections to standing documents and memory

The gap is not namespace-specific or callback-wide. Same-line assignment/query
ownership introduces false parameter Uses; multiline/declaration controls flow.
The initial broader const-initializer hypothesis was falsified and narrowed using
the assignment-kind gate, not merely absence of a counterexample. Nested return
ownership is a separate source-backed issue; initializer signatures are not Uses.
Do not weaken the correct byte-containment guard. No memory edits authorized.

## 4. Open work

Recommend caller-contained rvalue query
captures as the first bounded repair, with assigned-arrow and shared-consumer
negatives before implementation. Genuine same-line occurrence identity and nested
callable execution scopes need separate designs. Static skipped CJS names remain
independent; no unknown-CJS, closure or React.FC expansion.

## 5. Invariants and traps — do not do these

- Audit dispositions and test-only reindexing are not production fixes.
- Preserve one explicit ignored desired-behavior regression with captured RED.
- Preserve exact byte/owner/parameter barriers and independent refusal controls.
- Prism nav stale43paths/truncated; current source and probes are authority.

## 6. Identifiers

| Item | Value |
|---|---|
| Base | 7bdf9981 |
| Evidence | /private/tmp/prism-namespace-flow-MhHdDf |
| Cache versions | unchanged CPG91/nav52 |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED AND CORRECTED — round2 requested FIX W2/S0 on the audit,
not production expansion. Both findings were closed enumerable assertion gaps;
root folded targeted fixes on the same artifact at the two-round cap and reran
focused classification/desired RED. No third independent approval is claimed.
Full Rust verification was refreshed after the test-only corrections; source/test
hashes remained unchanged. The two ignores per full Rust configuration are the
pre-existing `resolution_test::slice_elem_variant_reserved` and the new explicit
desired-behavior regression. The latter still fails when run intentionally and is
not a passed behavior. Three historical Node tests lack required real-sites.jsonl;
Python live adoption is intentionally skipped; Tier-A quick has no verdict.
Production defects remain intentionally unresolved in this classification increment.

**Questions the owner owes an answer to:** None for classification/proof work.
