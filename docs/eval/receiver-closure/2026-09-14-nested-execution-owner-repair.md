# Nested-callable execution-owner bounded repair

## Outcome and boundary

This slice implements the bounded runtime repair designed by the merged proof in
PR #315. JS/TS/TSX rvalue queries for an exact, indexed callable now keep that
callable's ordinary body reads while excluding nested callable names, parameters,
defaults and bodies. The whole-file inventory remains legacy-recursive, unsupported
or recovery callables refuse ownership-sensitive collection, and the existing
capture/fallback, parameter, occurrence, call and receiver pathways are unchanged.

The repair is based on merged main `9e40a376299a714ed11233a56a9e23554d40ca99`
(tree `21e9fc389518e9c7928869baf854bfc6ad1a4054`). Work occurred in the
controller-prepared checkout `/private/tmp/prism-nested-owner-proof-pr-dYl0Qn`.
Evidence is rooted at `/private/tmp/prism-nested-owner-repair-TfBvIW`; commit and
publication remain controller operations.

## Implemented design

`src/ast.rs` adds private `RvalueQueryScope` and `RvalueOwnerDecision` types.
Callable scope is admitted only for JS/TS/TSX nodes found by exact kind and byte
range in `all_functions()` with a valid body range. There is no display-name,
line-number or nearest-ancestor ownership guess. File roots and non-JS languages
retain `LegacyInventory`; unindexed, malformed or missing-body callables select
`Unresolved` and return no ownership-sensitive rvalues.

Capture acceptance and recursive descent remain separate:

- assignment, call and return captures must be `Owned` by the queried callable;
- the three value-walker families independently stop at nested callable boundaries;
- own signatures/defaults and nested signatures/defaults are excluded;
- eager siblings around a callback remain owned by the outer expression;
- a computed object-method key is visited before refusing the method body;
- erased type contexts retain their existing refusal;
- whole-file root queries preserve recursive inventory;
- class heritage, field/static/block phases retain legacy behavior pending the
  separately scoped class-phase proof.

The public rvalue method signatures and general-purpose identifier collectors are
unchanged. `compute_param_def_nodes`, `argument_var_node_in_span`, occurrence
indexing, call/receiver ownership, navigation cache 52, and the separate
capture/fallback pathway were not loosened or replaced. Because stored DFG facts
change, the CPG cache was bumped from 92 to 93.

## Fail-first evidence

The unchanged merged production source was exercised in the same base harness and
environment before repair attribution:

| Control | Selected | Unchanged base | Candidate |
|---|---:|---|---|
| aggregate desired-owner contract | 1 | behavioral RED: 387 mismatch rows | 1 passed, exact eight-row required vector |
| nested default classification | 1 | behavioral RED: `outer=1`, `own_callable=1` | `outer=0`, `own_callable=0` |
| unindexed generator refusal | 1 | behavioral RED: nonempty rvalues | empty query/manual results; root inventory preserved |
| DFG census same-line arrow | 1 | behavioral RED: one outer-owned edge touched the arrow body | zero such edges; unrelated outer edges remain |

The aggregate mismatch stream is retained at
`base-desired-red.normalized`: 387 rows, SHA-256
`b96b7274a05149039d85672a3fac343ddc84838220fa74f1756c5cf17f83320b`.
Compilation mistakes and setup failures were excluded from RED evidence.

## Exact repaired observations

The final raw vectors contain assignment 2 rows, initializer 2, call 14 and
returned 2. Their normalized log hashes are recorded in the adjacent receipt.
The enabled desired contract's required positive vector is exactly:

```text
ManualNames|outerRead|1|-|-|-
ManualPaths|outerRead|1|-|-|-
ManualSpans|kept|1|48|52|function_declaration:0-54
ManualSpans|outerRead|1|31|40|function_declaration:0-54
QueryNames|outerRead|1|-|-|-
QueryPaths|outerRead|1|-|-|-
QuerySpans|kept|1|48|52|function_declaration:0-54
QuerySpans|outerRead|1|31|40|function_declaration:0-54
```

The representative graph now has exactly nine `Use` occurrences:

| Owner | Path | Line | Bytes | Confidence/source |
|---|---|---:|---:|---|
| inner | `p` | 4 | 67–68 | occurrence |
| inner | `seed` | 5 | 82–86 | occurrence |
| inner | `sink` | 4 | 62–66 | occurrence |
| outer | `cb` | 2 | 23–23 | occurrence |
| outer | `cb` | 3 | 33–33 | occurrence |
| outer | `local` | 8 | 120–125 | occurrence |
| outer | `seed` | 1 | 0–0 | occurrence |
| outer | `seed` | 5 | 71–71 | occurrence |
| outer | `seed` | 7 | 93–93 | occurrence |

The former outer `p@53–54`, `p@67–68`, `sink@62–66` and nested-body
`seed@82–86` rows are absent. Two conservative `NameOnly(CfgIncomplete)` labels
remain pinned to the outer `seed` line-5 zero-width anchor and do not become Exact.
The exact source epochs remain full/incremental-equal with DFG row counts
`5,5,7,6,5`; epochs 1–3 deliberately retain the separately parked duplicate
call-owner rows. FullFlow remains `[app.js:3:true, app.js:5:false]` with and without
DFG, so this source-level repair is not claimed as a changed public taint result.

## Verification

Focused ownership: 9 passed, 0 failed, 0 ignored. Separate desired invocation:
1 passed. Namespace-flow: 5 passed. Contained-rvalue: 3 passed. Full Rust default:
4,510 passed, 0 failed, 1 ignored. MCP: 4,703/0/1. The widest
`mcp detached-owner-audit` run: 4,703 passed, 23 failed, 1 ignored; the complete
23-name set is byte-identical to the unchanged-base set at SHA-256
`95694aa34ec43f56b0561c3af299f0b70d26aee3d73de1ac124957d10fa245cc`.
Every failure is the inherited explicit-compiler `NotPresent` population (21
library, 2 integration). Examples pass 32/0/0 after the fail-first census assertion
was updated to the repaired contract.

Node passes 786 tests across 46 modules at concurrency 2 after one environmental
retry restored three hash-pinned grammar archives from existing local custody.
The first attempt's one setup failure is preserved. Authority passes 40 rows.
Python initially failed before collection on the sandbox status socket; its one
environmental retry collected 938 passes and three skips, and the two real-binary
skips passed in a targeted supplement. Total covered Python tests are 940 passes;
the only remaining exclusion is live adoption requiring
`PRISM_RUN_LIVE_EVALS=1`.

`cargo fmt --check`, `git diff --check`, and all-target clippy completed. Clippy
reports the repository's existing warnings but none in the repair region or changed
tests. Tier-A matrix did not execute: the initial empty cache could not download and
the single offline retry lacked the platform `grpcio` wheel. Tier-A quick launched
against the freshly built release binary but timed out at 300,009 ms with no report.
Both Tier-A results are `INVALID`, not green. No full multi-corpus run occurred and
no baseline was changed.

The sole Rust ignore is
`resolution_test::slice_elem_variant_reserved`. The former desired-owner ignore is
enabled. The Node source-custody module
`docs/eval/receiver-closure/audit-imported-props-source.test.mjs` remains excluded
(three tests) because its five `PRISM_AUDIT_*` inputs, including
`/private/tmp/prism-imported-alias-O4d6E1/real-sites.jsonl`, are absent.

## Remaining semantic decisions

This repair does not solve genuine same-line occurrence identity; broaden
optional/default/rest/destructured or unparenthesized-arrow admission; define class
execution phases; resolve IIFEs/higher-order calls; broaden member shapes; or change
call/receiver ownership. Class-phase policy and the performance cost of exact
`all_functions()` membership checks remain explicit follow-up questions. The proof
and repair fixtures remain synthetic; no real-site recall or multi-corpus accuracy
claim follows.

## Review and custody

The declared review cap was two rounds. Independent round 1 matched the frozen tar,
all 11 source hashes and all four executable hashes, replayed ownership 9/0,
namespace 5/0 and the corrected example 1/0, and returned `APPROVE` with 0 WRONG
and one non-blocking SMELL; round 2 was not required. The retained SMELL is that
the example asserts more than one outer edge remains rather than naming the exact
`use(z)` endpoint. The main graph harness pins exact endpoint tuples, so no
candidate incorrect result was demonstrated. Review is at
`/private/tmp/prism-nested-owner-repair-TfBvIW/review-round1.md`.

The adjacent JSON is the machine-readable receipt. Under separately granted
controller authority, implementation commit `0858c23e` was pushed on
`feat/nested-callable-owner-repair` and PR #316 was opened against `main`. This
post-review documentation reconciliation does not alter the independently
reviewed implementation. Merge, live adoption and a full multi-corpus run remain
separate operations without authorization in this lane.
