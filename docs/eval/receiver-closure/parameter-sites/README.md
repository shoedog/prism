# Required-parameter real-site value checkpoint

Successor: the [bounded loop-header Def repair](../2026-09-10-js-ts-loop-header-defs.md)
is implemented and verified. Tables and classifications below retain the PR310
audit's historical measurements; exact callee-parameter-token binding remains next.

PR309 delivered substantial measured value. Compare pre-repair production
`f369f20cdcd3463bae86b191bcdd8ec9efd8f6fd` with merged production
`013d4bf014abe1c33449629854f8f8915d4237ef`, using byte-identical standalone
observers and identical Git archives in the same environment. This slice changes
only research tooling/tests/docs, not production resolution or cache authority.

| Observation | Public Excalidraw before → after | Authorized private frontend before → after |
|---|---:|---:|
| Loaded JS/TS/TSX files | 628 → 628 | 1,122 → 1,122 |
| Production-inventoried functions | 8,614 → 8,614 | 12,713 → 12,713 |
| Parameter syntax entries | 8,100 → 8,100 | 6,513 → 6,513 |
| Occurrence API tokens | 72 → 7,056 | 2,540 → 2,863 |
| Exact-token DFG and CPG parameter definitions | 37 → 3,892 | 1,467 → 1,676 |
| Use→Def flows targeting retained slot tokens | 12 → 6,016 | 2,123 → 2,420 |
| Use→Def flows unmatched to retained slot tokens | 352 → 9 | 68 → 68 |
| All observed Use→Def flows | 364 → 6,025 | 2,191 → 2,488 |

Public flow multiset: 6,004 added / 343 removed; private: 297 added / 0 removed.
These are graph observations, **not compiler-verified recall, unique source-call
counts, or new executable/receiver authority**. Nested ownership can repeat a
token under enclosing and nested functions. Slot membership alone is not proof
of a correct argument mapping. Missing slots alone do not prove a body target.

The fixed population includes loader skips (584 public / 140 private), source
hashes, language, parse-error counts, and static function/parameter/slot shapes.
`all_functions()` is the production inventory, not a raw exhaustive callable AST
census. Null owner names remain visible (4,497 public / 7,841 private); named
arrows and function expressions can still acquire owners through supported
contexts. Field-only uses do not necessarily produce a bare parameter Def.
Recovery syntax is retained as observation, not admitted as valid syntax.

## Residual source-backed classification

All 9 public and 68 private candidate unmatched observations were matched to
their source bytes, after checking each selected file against its census SHA-256.
Pinned TypeScript 5.9.3 parsed all 3 public / 6 private target files without syntax
diagnostics. Classification traversed syntax only (no Program, dependency install,
or semantic authority), converting UTF-16 parser offsets to UTF-8 byte offsets.
Seven local controls covered parameter, declaration, assignment, for-in, for-of,
Unicode-prefix and ordinary-read cases. Private rows and source remain local.

| Exact target syntax | Public flows (unique target tokens) | Private flows (unique target tokens) |
|---|---:|---:|
| Local declaration binding | 3 (3) | 2 (1) |
| Assignment target | 0 (0) | 59 (2) |
| Loop iterable read | 6 (2) | 7 (4) |
| Parameter binding / unclassified | 0 / 0 | 0 / 0 |

### WRONG: loop iterable reads become definitions

Both `extract_for_in_lvalues` and `extract_for_in_lvalue_spans` in `src/ast.rs`
scan all direct children instead of the loop's `left` field. For
`for (const item of items)`, the plain-identifier RHS `items` becomes a Def.
The recursive span gap pass feeds it into `DataFlowGraph::build`; this is not
caused by alias resolution (simple-binding loops emit no destructuring aliases).
Direct member/complex RHS expressions are outside this demonstrated helper case.

Public examples at Excalidraw commit
`0642e72cfa2d9a71198200e52f37399384610ee3`:

- [restoreLibraryItems, line 1113](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/data/restore.ts#L1113):
  five `Exact` flow observations target the iterable read `libraryItems`.
- [flatten, line 5](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/scripts/build-locales-coverage.js#L5):
  one `Exact` flow observation targets the iterable read `object`.

### WRONG: positional binding selects later body definitions

CPG Step 5b (`src/cpg/build.rs`) obtains parameter slot names, then searches every
line of the callee for the first same-path Def. Without the parameter-token Def,
it can connect an argument to a later assignment or alias-resolved local
declaration. Resolution confidence is copied onto that wrong endpoint.

At [store.ts, lines 216–257](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/element/src/store.ts#L216),
three `Exact` observations target local `storeChange`/`storeDelta` declaration
tokens at lines 223, 224 and 257, rather than the defaulted parameter bindings.
The six public loop-read flows above combine both defects. Both mechanisms and
these public residuals are present in the same-environment pre-repair control;
they are not regressions introduced by PR309. The source classification describes
candidate targets; it does not attribute every historical unmatched edge.

## Next recommendation and required negatives

The first step, bounded JS/TS/TSX loop-header Def repair, is now complete in the
linked successor. Its proof requirements were: both path and byte-span inventories
must retain left-hand bindings and reject right-hand reads.
Cover for-in/of (including await), bare/declaration/destructured left sides,
member/complex RHS, multiline/Unicode, and non-JS controls. Confirm the RHS cannot
become a CPG Def or interprocedural target. Do not remove legitimate RHS Use nodes.

Then separately repair callee argument binding to require an exact supported
parameter-token Def, not a body name search. Preserve parameter slot positions,
duplicates/recovery/write/cache barriers and existing non-JS contracts. Negative
fixtures must include the following forms, with required-parameter positives:

```ts
function loop(items: unknown[] = []) {
  for (const item of items) consume(item);
}
function overwrite(value: unknown = 0) {
  value = clean();
  consume(value);
}
function localAlias(value: unknown = 0) {
  let local;
  local = value;
  consume(local);
}
function caller(input: any) {
  loop(input); overwrite(input); localAlias(input);
}
```

Call arguments must never target the iterable read, later overwrite, or local
declaration in place of a parameter. Use RED-first production regressions in
those repair slices; the current native audit test characterizes fallback rather
than declaring that behavior correct. Do not widen optional/default occurrence
support to hide these defects. Public syntax counts include 194 optional and
249 default identifiers; private counts include 0 optional and 259 default
identifiers. Their eventual value checkpoint follows correctness repair.
No React.FC, closure admission or unresolved react-scripts decision changes.

## Reproduction and evidence limits

Build `cargo build --offline --release --example parameter_site_census` against
each production artifact with identical example bytes, freeze both binaries, and
run on the same owner-approved immutable root. Bound each subprocess to 15 minutes
and 128 MiB output. Raw stdout contains source identifiers: keep private output
local. Compare with `node docs/eval/receiver-closure/parameter-sites/compare.mjs
base.json candidate.json`; this prints aggregate data only. Run the comparator's
tests with `node --test docs/eval/receiver-closure/parameter-sites/compare.test.mjs`.

Prism navigation reported a stale index (41 changed paths), so current source
tracing—not exhaustive navigation—established the producer/consumer distinctions.
Native six-test GREEN and meaningful classifier RED are captured. Comparator
round-one review found two bounded defects: body occurrences were allowed as
parameter tokens, and unmatched slots were mislabeled as non-parameter evidence.
The former is now rejected; the latter is explicitly classified from source.
Initial missing-module RED is not behavioral evidence; targeted validator RED is.
Round two independently found the code and measurement claims clean; its sole
WRONG was the handoff's stale checkpoint/dirty-state entry. That bounded
documentation correction is reconciled in closeout, not a third review round or
an unconditional independent approval of the final documentation. The round cap
was two; findings converged from two code/label defects to one custody entry.

## Final verification

The full runner started and ended on clean
`cda0b972fcd8cf253ae2c45546a33c59e8970d3c`; later changes are documentation only.

| Gate | Result |
|---|---|
| Rust default / MCP / MCP+detached-owner-audit | 4,123 / 4,316 / 4,339 passed; one existing ignore each |
| Observers / receiver helpers / authority controls | 726 / 18 / 40 passed |
| All native examples | 32 passed, including 6 new audit tests |
| Comparator | 10 passed; same-environment pre-fix control 7 passed / 3 failed |
| Python | 940 passed; one intentional live-adoption skip |
| Formatting / diff | passed |

Four individual comparator controls also prove each added token/Def invariant
rejects a state incorrectly accepted by the pre-fix validator. All 77 candidate
unmatched flow observations exist identically in the base multiset. Both controls
are retained locally, along with the original raw runs and final comparisons.

The first Python run had 939 passes / 1 failure / 1 skip: the existing prewarm
test recognizes only binary paths ending in `prism`, while the initially frozen
binary was named `prism-python`. The identical base test/module reproduced the
failure in the same environment. Byte-identical binaries under normal names
gave the full 940-pass result. No production/test fix or silent rebaseline was
made; the initial failure, base control and corrected invocation are retained.

Rust's ignore is `resolution_test::slice_elem_variant_reserved`; Python's live
adoption module remains explicitly opt-in. Tier-A is not triggered: production
AST/CPG/navigation paths are unchanged. No Clippy, live-model, full multi-corpus,
or compiler-Program accuracy evaluation was run in this audit. No runtime recall
or complete receiver-coverage claim follows from these green gates.
