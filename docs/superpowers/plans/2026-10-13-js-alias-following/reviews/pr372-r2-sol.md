# PR #372 — confirmation review, round 2 of 2

**FIX. Five WRONG / MATERIAL findings remain.** The original counterexamples are repaired, but the fold does not establish the tightened promise. Whole-pattern admission still admits an early-error pattern; the callable route and textual stand-down have additional non-proven effects; the formatter adds a false caller across files. The taint trace is another spelling-keyed consumer that disagrees with a proven alias edge.

These are closed, enumerable mechanisms with bounded repairs on this artifact. This is the declared final round: no restart, implementation dispatch, or extra review round was performed. Preserve this artifact and fix the listed guards/consumer lookups. No SMELL is used as a blocker.

## Binding and evidence

Reviewed the clean `/Users/wesleyjinks/code/prism-js-alias`, branch `feat/js-alias-follow`, HEAD `d22f8349ec22e17778efae0cee0df40253ab17f3`. Used `origin/main...HEAD` (merge base `86756aadeec8baf5a3185bfdc38de10256d2ebfa`) and the specified `3dd5b7db..HEAD` fold range; unrelated MCP changes were excluded. Read both `sol.md` and `opus.md` round-1 reports.

The supplied prebuilt releases report `d22f8349ec22` and `38a599948771`. SHA-256 and test commands are in [verification.txt](verification.txt). Their build provenance beyond the supplied paths, version strings and measured hashes is inherited. Every behavioral comparison ran head and main on the same scratch fixture, in the same environment, with separate fresh scratch caches. The scripts and complete output artifacts are retained alongside this report. Exit status alone is not evidence. No fixture/corpus package was executed; `node --check` was syntax-only.

Live [PR metadata](https://github.com/shoedog/prism/pull/372) agrees with this head. The latest attempt of [CI run 38492020022](https://github.com/shoedog/prism/actions/runs/38492020022) has nine successful jobs. Read-only GitHub connector calls replaced sandbox-blocked shell network commands. I did not inspect failed-attempt logs or independently attribute the MCP flake.

## Findings — WRONG first

### W1 — WRONG / MATERIAL: duplicate sibling BoundNames still admit an invalid pattern

This is **sol W1 left partially open**, not a new finding class. All four original duplicate-target patterns are repaired; a duplicate of another name in the same declarator still bypasses admission.

`fixtures/pattern-dup-other/app.ts`:

```ts
import * as N from './m';
export function run() {
 const { f, x: a, f: a } = N;
 return f();
}
```

`m.ts` exports ordinary functions `f` and `x`. The declaration binds `a` twice and is an early SyntaxError. Head emits **Exact import_member → m.ts:f@1-1** for line 4; main emits **Source / UnknownName**, no target. The selected `f` is not duplicated, but the declaration itself is invalid. This is the same early-error admission problem as round 1, rather than a runtime-initialization objection to static Exact.

**Evidence:** [head rows](pattern-dup-other-head.jsonl), [main rows](pattern-dup-other-main.jsonl), [syntax diagnostic](pattern-dup-other-syntax.txt). A scratch `.mjs` copy checked with `node --check` reports `SyntaxError: Identifier 'a' has already been declared`. In [js_binding_alias.rs:161](/Users/wesleyjinks/code/prism-js-alias/src/ast/js_binding_alias.rs:161), every entry's *shape* is admitted, but only `bound == name` contributes to `keys`. The existing declaration collector deduplicates BoundNames per declarator, so the usual multiple-declaration check does not expose the duplicate sibling. This distinguishes a semantic early error from tree-sitter recovery.

**Recommended fix:** During the whole-pattern pass, retain every bound identifier in a set and return `None` on any repeated bound name, including names other than the queried one. Keep the existing shape restrictions and selected-key check. Add shorthand/shorthand, shorthand/pair and pair/pair duplicate-sibling negatives, with a valid repeated-property-key/different-bound-name control. The reported fixture must equal main's complete records.

**Alternative:** Make the general pattern declaration collector occurrence-sensitive and refuse the whole invalid declarator. Cost: broader binder semantics and regression review, beyond this small link-admission repair.

**Does not apply:** A wholly admitted pattern with unique BoundNames. Repeating a property key while binding different identifiers is valid and should remain followable. Escaped binding identifiers are already refused by the scope-clean guard.

**Confidence: 100/100.** The paired outputs and syntax diagnostic establish the overclaim. A complete BoundName uniqueness check would collapse it.

### W2 — WRONG / MATERIAL: the callable arm still returns an unproven drop instead of main

This is an incomplete application of the revised fallback contract, not a request to support non-JSX calls to React wrappers.

`fixtures/wrapped-local/app.ts`:

```ts
import { memo } from 'react';
const C = memo(function Comp() { return null; });
function h() {}
export function run() {
 const h = C;
 return h();
}
```

At line 6, head emits **Source / WrappedExportNonJsx**, no target. Main emits **Exact local_def → app.ts:h@3-3**. There is no proven alias-route hit on head, so the new refusal violates the promise even though the legacy edge being removed is wrong. Removing the unrelated `function h` still demonstrates the mismatch: head drops `WrappedExportNonJsx`; main drops `UnknownName`.

**Evidence:** [with-decoy head](wrapped-local-head.jsonl), [main](wrapped-local-main.jsonl), [without-decoy head](wrapped-local-nodecoy-head.jsonl), [main](wrapped-local-nodecoy-main.jsonl), [counter](counts-wrapped-local-nodecoy.json). The counter is `callable_unresolved: 1`, excluding the alternative that the binder did not follow the alias. [resolution_js_alias.rs:46](/Users/wesleyjinks/code/prism-js-alias/src/resolution_js_alias.rs:46) unconditionally wraps `js_ts_local_callable` in `Some`, including its drops. [resolution.rs:3413](/Users/wesleyjinks/code/prism-js-alias/src/resolution.rs:3413) then returns that drop before legacy rungs. The import and namespace arms correctly return `None` when they cannot prove a hit; the callable arm does not.

**Recommended fix:** Return `Some` for `AliasCallable` only when the outcome is the singleton Exact target; otherwise return `None` and continue main's rungs. Coordinate with W3 so main's textual records also remain. Pin both wrapped fixtures and the same-line identity collision below. Update the counter's unresolved/fallback labeling and the stale module-level comment, which still says no candidate means dropped.

**Alternative:** Explicitly broaden the contract to permit callable-terminal refusals and enumerate their record differences. Cost: owner approval of a different slice boundary, plus changed docs, fixtures and measurements; this cannot be described as confirmation of the supplied promise.

**Does not apply:** A callable route yielding one Exact target, including an eligible JSX wrapper site. Direct `Callable` behavior remains outside this repair.

**Confidence: 100/100.** The drop-reason-only fixture independently refutes record equality without relying on a legacy wrong edge. Singleton-hit gating would collapse this finding.

### W3 — WRONG / MATERIAL: stand-down removes other unproven-terminal records

The disclosed span-less named-import case is **not the complete exception population**. This is the binding-kind/resolution-time seam called out in the brief.

`fixtures/ns-unproven-local-decoy/producer.ts`:

```ts
export function f() {}
export const x = f;
```

`app.ts`:

```ts
import * as N from './producer';
function decoy() { function N() {} }
export function run() {
 const { x: h } = N;
 return h();
}
```

For line 5, main emits **Source h / UnknownName**, plus **IndirectResolution N / Exact local_def → app.ts:N@2-2**. Head emits only the dropped Source record. The export `x` is an unproven exported alias, so there is no proven alias-route hit. This is a namespace destructure, not the disclosed span-less named import. The removed edge is wrong, but the positive-only promise explicitly preserves main's records outside proven hits and its named exception.

There are also differences without a removed edge: `fixtures/ns-unproven-text` and `fixtures/ns-path-text` have an unrelated cross-file `N`, and main's extra textual record itself drops `UnknownName`; head still removes that record. For `fixtures/local-span-collision`, `function f() { function f() {} }` gives two same-name/same-line identities: head cannot prove the local terminal and removes main's textual record containing both Exact candidates.

**Evidence:** [namespace head](ns-unproven-local-decoy-head.jsonl), [main](ns-unproven-local-decoy-main.jsonl), [unproven counter](counts-ns-unproven-text.json), [collision head](local-span-collision-head.jsonl), [main](local-span-collision-main.jsonl), and the analogous `ns-unproven-text-*` / `ns-path-text-*` files. [call_graph.rs:2762](/Users/wesleyjinks/code/prism-js-alias/src/call_graph.rs:2762) suppresses Level 1 for every alias variant regardless of route outcome. Its current comment's claim that the textual record is a duplicate or wrong edge also misses the dropped-record cases. `namespace_member_kept_main: 1` describes the Source fallback, but does not mean the site's complete records equal main.

**Recommended fix:** Stand down on a proven alias-route hit, rather than merely a followed binding kind. That keeps one record for successful alias resolution and preserves the entire base record set on failure. Retaining the specifically disclosed span-less named-import exception requires an explicit narrow predicate/test; restoring that base record instead removes the exception altogether. Pin complete origin/target/drop records for these fixtures and the collision; do not assert only Source equality.

**Alternative:** Keep the current guard and obtain owner approval for all non-proven textual-record removals as a deliberate refusal policy. Cost: a broader exception to the slice, disclosures and additional measurements; it is not the contract supplied for this review.

**Does not apply:** A proven alias hit, an unfollowed alias, or a site for which main's textual pass would never create a record. The span-less named-import case is already disclosed and is not re-raised as new.

**Confidence: 100/100.** Full paired records establish the difference, and the counter excludes a successful namespace route. Proof-gated stand-down would collapse it.

### W4 — WRONG / MATERIAL: `--format callers` attaches a proven alias to the wrong file

This is **new in the fold**. Navigation's incoming index repair is sound; the formatter's additional alias scan loses target identity.

`fixtures/format-callers-target-file/` contains:

```js
// m.js
export function f() {
 return 1;
}
// b.js
export function f() {
 return 2;
}
// app.js
import * as N from './m';
export function run() {
 const h = N.f;
 return h();
}
```

Use [format-all.diff](format-all.diff), which includes all three files in the review universe. Head `--format callers` lists **app.js:run, call line 4** as a caller of **both m.js:f and b.js:f**. Main lists no caller for either. The added caller of `m.js:f` is correct; the added caller of unrelated `b.js:f` is wrong. Head's site dump has exactly one Exact target, `m.js:f@1-3`; `nav callers --symbol f --file b.js` is empty on both binaries.

**Evidence:** [head formatter](format-all-head.json), [main formatter](format-all-main.json), [head resolved site](format-b-head-stats.json), [head b.js navigation](format-b-head-nav.json). [review.rs:379](/Users/wesleyjinks/code/prism-js-alias/src/output/review.rs:379) reduces resolved targets to `BTreeSet<String>` names; [review.rs:436](/Users/wesleyjinks/code/prism-js-alias/src/output/review.rs:436) appends sites whenever that set contains the walked name. Two files exporting `f` therefore share the new alias callers. Main's raw bucket did not contain this namespace alias, so this false caller is introduced by the append, not inherited. An initial b.js-only CLI diff omitted the alias and its module from the loader; that formatter observation was inadmissible and replaced by the all-file probe.

**Recommended fix:** Preserve `FunctionId` in the collected alias targets and in the BFS queue; append a followed-alias site only for the exact current callee identity. Retain existing raw-name behavior for non-alias sites if that is the intended compatibility boundary. Test distinct files and same-name nested functions, plus multi-hop aliases and the same-spelling once-only case.

**Alternative:** Use a shared resolved reverse index for all formatter traversal. Cost: a broader behavior change to legacy raw callers, with corresponding compatibility tests; it is cleaner long term but unnecessary for this bounded repair.

**Does not apply:** A review universe containing only one function of the walked name, or consumers using the full target identity, such as the repaired `nav callers` index. Unchanged raw-name false callers are follow-ups, not this finding.

**Confidence: 100/100.** Both base control and the exact-target navigation control separate the new formatter defect from legacy resolution. Full-identity matching would collapse it.

### W5 — WRONG / MATERIAL: taint call descent still selects sites by target spelling

This answers the undisclosed-consumer question. It is an omitted consumer of the new alias argument edges, not a claim that the fold changed `trace.rs`.

`fixtures/trace-alias/app.js`:

```js
function sink(p) {
 const y = p;
 return y;
}
export function run() {
 const x = input();
 const h = sink;
 h(x);
}
```

Head's line-8 call resolves **Exact local_def → sink@1-4**, and `nav callers sink` correctly reports `run`. Nevertheless:

```text
nav taint-reaches --repo <fixture> --source app.js:6 --sink app.js:2 --format json
```

returns **BoundaryExited**, stopping at parameter `p`. Changing only `h(x)` to `sink(x)` produces **Reached** with a `CallDescent` witness on **both head and main**. Main on the alias fixture reports **NotReached**: it has the textual call edge but lacks the alias site's argument transfer. This is a newly reachable argument edge whose downstream exact-descent consumer still ignores its resolved identity; it is not an assertion that head loses a taint result main already had.

**Evidence:** [head alias trace](trace-alias-head-trace.json), [main alias trace](trace-alias-main-trace.json), [head direct trace](trace-direct-head-trace.json), [main direct trace](trace-direct-main-trace.json), [head alias site](trace-alias-head-stats.json), [head alias callers](trace-alias-head-callers.json). The boundary at `p` establishes that the argument edge exists, ruling out missing Step-5b construction as the explanation. The direct controls also have ordering warnings yet reach the sink; those warnings do not explain this difference.

[trace.rs:734](/Users/wesleyjinks/code/prism-js-alias/src/cpg/trace.rs:734) obtains candidate sites from `call_graph.callers.get(target_fn)`. The new Source site is filed under `h`, so that bucket cannot prove descent into `sink`. The sibling self-parameter boundary classifier at [trace.rs:665](/Users/wesleyjinks/code/prism-js-alias/src/cpg/trace.rs:665) has the same name-key assumption. My self-alias probe did **not** produce a false Reached result; no such result is claimed here.

**Recommended fix:** Select source-occurrence sites from the actual caller's `calls` bucket, independent of callee spelling, then preserve the existing occurrence-uniqueness and Exact target/file/span checks. Apply the same identity-aware selection to the self-parameter check. Add an alias/direct pair requiring cross-function Reached, a recursive alias boundary control, and the existing ambiguous-occurrence/depth negatives. This does not require relaxing any descent guard.

**Alternative:** Supply trace with a resolved reverse index that preserves target and source occurrence identities. Cost: additional construction/cache ownership and a wider refactor; indexing raw callers under a second spelling would affect every reverse-map consumer and duplicate handling.

**Does not apply:** Sites whose spelling equals their target name, no argument DataFlow edge, or a deliberately refused ambiguous/depth-limited descent. The fixture has one ordinary cross-function call and no such ambiguity.

**Confidence: 99/100.** Paired trace output, its boundary location and the exact resolved edge establish the lookup failure. An identity-aware site selector yielding the direct twin's witness would collapse it; mere removal of an ordering warning would not.

## Closure of the supplied rows

| Round-1 row | Confirmation |
|---|---|
| Duplicate/rest/nested/computed/string patterns | All four original bad inputs now equal main; valid rest/nested siblings also equal main. Whole-pattern shape admission is correct, but the invalid-declarator defect remains for duplicate other BoundNames: W1. |
| Unproven export alias; paths/CJS namespace terminal | Original named/member exported-alias and nested paths/CJS inputs equal main, with complete records compared. No new paths/CJS support was added. The universal fallback promise remains false for W2/W3. |
| Span-less export / nested decoy | Both import alias routes require a span; namespace fixture equals main. Named-import fixture has exactly the disclosed extra-record removal. Direct routes are unchanged. |
| Callers | `nav callers` repaired: import, renamed import, member, shorthand, renamed destructure, same-spelling and same-file aliases each count once. Original formatter omission is repaired in the unique-name tests, but its new file-identity error is W4. The disclosed `callers_of` / `resolve_callers` limitations remain. |
| End span, stand-down mutants, nested-sibling unit row | End-only collision test, whole-pattern negatives and the listed mutant entries are present; selected tests pass. The lane has 41 entries. Its all-killed execution result is inherited, not independently rerun. |
| Counters | The terminal/outcome counters exist and the full-value test passes. Additional probes confirm `callable_unresolved`, `namespace_member_kept_main`, and `import_proven`. `kept_main` currently describes Source fallback only, as W3 shows. |
| CLAUDE wording | The old “only” assertion is removed and the qualification is appropriate to the intended contract. W2/W3 prevent the implementation from satisfying that qualification universally. |
| PR description / argument flow disclosure | Field-only labels are corrected, whole-pattern exclusions are stated, and argument flow is disclosed. Counts match the pinned matrix. The universal promise and “every input” claim need reconciliation with W1–W3, and the consumer disclosure needs W4/W5. |
| Explicit follow-ups | Not reopened: flat caller-locals guard, redundant visited set, direct span-less-export fallback, and `callers_of` / `resolve_callers`. |

[closure-comparisons.json](closure-comparisons.json) records twelve head/main fixture comparisons: eleven equal; only the disclosed span-less named import differs. [matrix-summary.json](matrix-summary.json) recounts 1,169 base rows, 1,335 head rows, 166 new rows, 239 MISSING→CORRECT, 39 WRONG→CORRECT and 25 same-verdict field changes. The complete pinned matrix test passes. These denominators do not include the additional counterexamples above.

The nav-index bypass uses a literal count of one only when the alias route has a hit; same-spelling sites do not also take the name-count branch. Non-alias sites retain their old branch. The Source sites are filed once in the reverse map. I found no defect in that counting repair. Both import alias routes' mandatory spans and their singleton checks are sound; the direct import route retains `alias_route=false` behavior. The per-export namespace proof remains preferable to the legacy qualified stem fallback.

Other source-reviewed spelling consumers include diff-scope selection (`cpg/context.rs`, raw caller names and `resolve_callees`) and collision-location diagnostics. The diff-scope skeleton deliberately omits indirect resolution on main too; I have not demonstrated a new fold-caused regression there. It is a follow-up rather than a new WRONG. W5 is the additional executed consumer disagreement. W4 is a defect in the consumer the fold itself changed.

## Checked and found sound

- Checkout/head/base binding, clean tree, whole-PR and fold scopes; latest PR head and nine latest CI job statuses.
- Original reported pattern negatives and valid forbidden siblings; comment admission; original unproven-export, paths and CJS fallback inputs, including full origin/target/drop rows.
- Span requirement on both import alias routes; per-export namespace routing; independent end-line identity test.
- Nav incoming-index once-only repair, including alias spelling equal to target and unchanged direct-site behavior.
- The unique-name formatter inclusion test and counter full-value assertions, subject to W4 and W3 respectively.
- Existing cache version/serialization changes by source inspection; no new persisted shape in the fold.
- **161 distinct selected tests passed, 0 failed**: 90 `--lib js_binding` (includes 17 alias tests), 71 `--test integration js_alias` (includes full pinned matrix). The 17 alias tests also passed separately. All ran offline with bless/print overrides unset and temporary files directed into scratch. Two unrelated integration unused-code warnings were reported; no attribution was made.
- Matrix totals and 41 mutation entries; all five counterexamples are source-bound and have same-environment main controls. See [hypothesis-log.md](hypothesis-log.md) for alternatives and probe results.

## Not checked

- Full project suite, release rebuilds, Tier-A runs, actual mutant executions, CI failed-attempt logs, or independent binary build provenance. This is a bounded confirmation review, not an implementation-done claim.
- Re-running X/T/F corpus aggregates, provider/compiler ground truth, runtime module execution, exhaustive scope/input space, incremental cache execution, or performance. No forbidden source content was read. An early AGENTS filename inventory used `rg --files` broadly under `/Users/wesleyjinks/code`; it may have traversed excluded directories. That inventory exceeded the intended directory boundary, and subsequent reads used scoped paths.
- A standalone alias-specific cache round-trip rerun, all changed matrix rows individually, or all downstream reasoning/algorithm consumers. The cache source and generic contracts were reviewed; that is not execution evidence.
- Fixes for these findings: no worktree edit, source mutation, Git write or GitHub post occurred. Scratch artifacts and this report are the deliverable.

FIX — round 2/2: reject duplicate sibling BoundNames; fall back for unproven callable routes; gate textual stand-down on proven hits; preserve formatter target identity; remove taint trace's callee-spelling gate.
