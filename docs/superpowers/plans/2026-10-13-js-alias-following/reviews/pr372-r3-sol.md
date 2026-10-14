# PR #372 — round 3 of 3, final confirmation

**FIX. Four WRONG / MATERIAL findings remain.** The round-2 examples are repaired. The stronger construction still fails at declaration admission, off-site textual records, and callers identity. There is also a bounded predicate defect for intrinsic JSX: a route hit need not be the Source record's outcome.

This is the owner-authorized final round. The first, third and fourth findings below are further instances of the existing families; the second is an additional, bounded site-eligibility mechanism. At the cap, those families remain open-class. Escalate the invariant/design boundary to the owner, preserving this artifact and its evidence; do not silently dispatch another review or restart the implementation. No owner-decided follow-on is raised here.

## Checkout and evidence

Reviewed the clean `/Users/wesleyjinks/code/prism-js-alias`, branch `feat/js-alias-follow`, HEAD `568c284f1a588f08d21a917eb091c88cea2e2a64`. Whole-PR comparison: `origin/main...HEAD`, merge base `86756aadeec8baf5a3185bfdc38de10256d2ebfa`; local `origin/main` is `e7dc1cc277f07eb85f5be25db2cf050d798a8a00`. Rework comparison: `d22f8349..HEAD`. Read all four earlier reports, the changed production code, the binder's scope/cleanliness/classification helpers, and the specified tests.

The supplied releases report `568c284f1a58` and `38a599948771`. SHA-256:

- Head: `788e385b64ab56586f1ce2a865776e3812e0cfd2362500ce08b1a110f66c405b`.
- Main: `9083763933fd7f4128e50541542c012849d07d4d0254ffe611b7d87102b14227`.

Their build provenance beyond the supplied paths, revision strings and hashes is inherited. Each finding uses both binaries on the same synthetic source in the same environment, with separate scratch caches; no regression attribution relies on another environment. No packages were executed. `node --input-type=module --check` only parsed my source. No release was rebuilt, worktree file edited, Git write attempted, or GitHub content posted.

Read-only GitHub connector calls replaced blocked shell egress. [PR metadata](https://github.com/shoedog/prism/pull/372) matches the reviewed head. [CI run 38500629044](https://github.com/shoedog/prism/actions/runs/38500629044) is successful, with all nine jobs successful. I did not inspect job logs or independently verify first-attempt history.

## WRONG findings

### W1 — WRONG / MATERIAL: flat, unique patterns still admit invalid declarations

**Existing admission family, not closed by uniqueness within one pattern.**

With `lib.js` exporting `function f() { return 1; }`:

```js
import * as N from './lib';
export function run() {
  const { f, x: eval } = N;
  f();
}
```

Head emits one Source record, **Exact import_member → lib.js:f@1-3**. Main emits **Source / UnknownName**, no target. The module declaration is an early SyntaxError. This concerns declaration formation, not runtime mutation or initialization. Strict-mode `eval`/`arguments` bindings are forbidden by the [ECMAScript identifier early-error rules](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-identifiers-static-semantics-early-errors).

The demonstrated population is broader than that spelling:

| Declaration before `f()` | Syntax-only diagnostic | Head | Main |
|---|---|---|---|
| `const { f, x: eval } = N;` | Invalid destructuring assignment target | Exact `lib.js:f` | UnknownName |
| `const { f, x: arguments } = N;` | Invalid destructuring assignment target | Exact `lib.js:f` | UnknownName |
| `const { f, x: let } = N;` | Unexpected strict mode reserved word | Exact `lib.js:f` | UnknownName |
| `const { f, x: a } = N, a = 1;` | Identifier `a` already declared | Exact `lib.js:f` | UnknownName |
| `const { f, x: a } = N; const a = 1;` | Identifier `a` already declared | Exact `lib.js:f` | UnknownName |

Duplicate names across a lexical declaration's BindingList are also an early error under the [lexical-declaration rules](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations-static-semantics-early-errors). The repeated-declarator example has unique names in the admitted pattern itself.

**Evidence:** [probe log](probes.log); complete paired outputs and syntax diagnostics under [fixtures/pat_eval](fixtures/pat_eval/), `pat_arguments`, `pat_let`, `pat_dup_declarator`, and `pat_dup_declaration`. The valid `{ f }` control is followed; `{ f, x: a, y: a }` and the escaped duplicate control both equal main. Thus escape decoding and parse recovery are not the explanation for the demonstrated admits.

**Mechanism:** [js_binding_alias.rs:155](/Users/wesleyjinks/code/prism-js-alias/src/ast/js_binding_alias.rs:155) validates flat entry shapes and uniqueness only inside the selected pattern. The existing scope classifier validates declaration multiplicity for the queried `f`, not each sibling name. It does not prove that the selected declaration forms a legal binding. Tree-sitter's clean parse is insufficient for semantic early errors.

**Recommended family closure:** Introduce one declaration-admission check shared by all alias-link shapes, independent of the queried name: validate the relevant bound identifiers in their strict/contextual grammar, occurrence-sensitive BoundNames across the lexical declaration, and declaration conflicts involving those names in the declaring scope. If that proof is unavailable, keep the alias as main does. Give this gate a stated semantic boundary and table-driven negatives plus valid controls; adding another rejected string or only widening the current pattern set does not close the family.

**Alternative:** Conservatively decline pattern following until that declaration proof exists, retaining identifier/member aliases that pass an explicitly defined admission gate. Cost: yield reduction for valid destructures, requiring owner agreement to the narrower feature. A broader binder-wide early-error validator would avoid that yield loss but carries a larger implementation/review cost.

**Does not apply:** A legal flat declaration with unique, contextually admissible BoundNames and no relevant declaration conflicts. Property-key repetition with different bound identifiers remains valid. This finding does not require rejecting valid aliases due to runtime TDZ behavior.

**Confidence: 100/100.** Paired records plus syntax diagnostics demonstrate the overclaim. A declaration-level proof, rather than a selected-name proof, would collapse it.

### W2 — WRONG / MATERIAL: the predicate proves a lowercase JSX alias that Source resolution drops

**Bounded additional mechanism: site eligibility is outside the predicate.**

`lib.jsx` exports ordinary `target@1-3`; the relevant caller is:

```jsx
import * as N from './lib';
export function run() {
  const { target: h } = N;
  return <h />;
}
```

The retained probe also has an unrelated same-named local function, with a distinct file/span, to check attribution.

| Consumer | Head | Main |
|---|---|---|
| Source record for `<h />` | `JsxIntrinsic`, no target | Same |
| Head predicate telemetry | `namespace_member_proven: 1` | No alias-follow counter |
| `nav callers target --file lib.jsx` | No caller | No caller |
| `--format callers`, touching `lib.jsx:target` | **Lists `run` at the element** | No caller |

The Source resolver correctly treats lowercase JSX as an intrinsic tag. The newly added formatter caller is wrong even under prism's own static-binding model. The promise's true branch also fails: the predicate is true while the Source record is dropped, rather than containing its one Exact target.

**Evidence:** [JSX probe log](jsx-probes.log), [head Source records](fixtures/jsx_pattern-lower/head-stats.jsonl), [main Source records](fixtures/jsx_pattern-lower/main-stats.jsonl), [head counter](fixtures/jsx_pattern-lower/head-counters.json), and paired `head-fmt.jsonl` / `main-fmt.jsonl` and nav output in that directory. Uppercase `<H />` proves and resolves the imported target, with the expected formatter caller. A plain `const h = N.target` reproduces the lower-case defect too.

**Mechanism:** [resolution.rs:2306](/Users/wesleyjinks/code/prism-js-alias/src/resolution.rs:2306) drops intrinsic JSX before entering the alias route. [resolution_js_alias.rs:43](/Users/wesleyjinks/code/prism-js-alias/src/resolution_js_alias.rs:43) only tests its terminal outcome; it does not apply that site gate. [review.rs:372](/Users/wesleyjinks/code/prism-js-alias/src/output/review.rs:372) calls the route directly, so it sees a proof the public resolver never returns. The callers index consumes the public resolution, explaining the disagreement without assuming a cache problem.

**Recommended fix:** Share the intrinsic/site-eligibility gate between the public resolver and the alias predicate. A site rejected before the alias rung cannot be a predicate hit; all three consumers must use that same eligible outcome. Extend the structural test's true branch to assert the public outcome equals the route's singleton Exact identity/kind with no drop. Add lowercase identifier/import/member/destructure JSX negatives and uppercase/`_`/`$` positive controls.

**Alternative:** Make each downstream consumer validate the public resolver's outcome before accepting a route hit. Cost: duplicated policy and a predicate that still mislabels intrinsic sites; the stand-down and telemetry would need coordinated changes too. Prefer fixing the common predicate.

**Does not apply:** An eligible capitalized JSX name or ordinary call reaching the alias rung. Do not change the existing intrinsic-tag behavior.

**Confidence: 100/100.** The true counter, dropped Source record, absent nav caller, and newly present formatter caller are independently observable. Predicate eligibility shared with the resolver would collapse the finding.

### W3 — WRONG / MATERIAL: formatter identities only filter the added list, leaving a false transitive caller

**Existing callers-identity family; original round-2 cross-file examples are repaired, but filtering is incomplete.**

`lib.js` exports `target@1-3`. `app.js`:

```js
import * as N from './lib';
export function run() {
 const h = N.target;
 h();
}
function decoy() {
 return 2;
}
export function outer() {
 const run = decoy;
 run();
}
```

A diff touching `lib.js:target` and the caller file's import line, with `--format callers --caller-depth 2`, yields:

| Consumer | Head | Main |
|---|---|---|
| Source `run()` inside `outer` | Exact `app.js:decoy@6-8` | Exact `app.js:run@2-5` |
| `nav callers run` | No caller | `outer` |
| `nav callers decoy` | `outer` | No caller |
| Formatter callers of `lib.js:target`, depth 1 | `run` | None |
| Formatter callers of `lib.js:target`, depth 2 | `run`, **`outer`** | None |

Head proves that `outer` calls `decoy`; nevertheless the formatter lists it as a depth-2 caller through the unrelated `run`. The alias edge newly makes that false transitive caller reachable. This is not a report of an unchanged, already reachable main edge.

**Evidence:** [final fresh-cache probe log](callers-probes-final.log), [depth-2 head](fixtures/callers_alias/head-fmt2.json), [depth-2 main](fixtures/callers_alias/main-fmt2.json), paired Source dumps and nav results in [fixtures/callers_alias](fixtures/callers_alias/). Replacing `h()` with direct `N.target()` causes **both** binaries' old formatter to list `outer`: that control locates the inherited bucket defect and shows why the newly followed route makes it reachable. Depth 1 excludes `outer`, isolating reverse traversal rather than initial target identity.

**Mechanism:** [review.rs:419](/Users/wesleyjinks/code/prism-js-alias/src/output/review.rs:419) copies all sites in `callers[callee_name]` into `sites`, without applying the current `FunctionId`. Only appended `alias_sites` are filtered at line 432. When the queue reaches `run@2-5`, the proven alias site spelled `run` is already present from that first list and bypasses identity filtering.

**Recommended family closure:** For every traversal step, remove predicate-true alias sites from the original spelling bucket and reintroduce them only for their proven `FunctionId`, using the same identity logic as the added list. Apply identity admission before the caller is credited or enqueued. Leave predicate-false sites on main's path. Cover both an added-list alias and a spelling-bucket alias, same/different file/span, depth 1 and depth 2, with a negative target and a genuine transitive positive.

**Alternative:** Traverse the resolved incoming index for callers output. Cost: broader changes to legacy callers and de-duplication behavior; isolate such a rewrite if preserving predicate-false behavior cannot be proved.

**Does not apply:** The legacy spelling bucket's site is predicate-false, or its proven target really equals the current identity. The direct-control false chain exists on main and is an inherited follow-up on its own; only newly reachable alias credit is this finding.

**Confidence: 100/100.** Source resolution and nav callers agree on `decoy`; only formatter traversal credits `run`. Filtering both candidate sources by the identity would collapse it.

### W4 — WRONG / MATERIAL: suppressing one alias record adds an indirect record at another, false-predicate site

**Existing record-parity/stand-down family; the local Boolean does not make the later passes site-independent.**

`app.js`:

```js
function target() {
 return 1;
}
export function run(obj) {
 const h = target;
 h(); obj.callback();
}
```

`other.js`:

```js
function otherTarget() {
 return 2;
}
function init(otherObj) {
 otherObj.callback = otherTarget;
}
```

At `h()`, the head's Source Exact target and removed duplicate textual record are allowed. At the separate `obj.callback()` span (bytes 85–99, line 6), the predicate is false: it is a qualified call on a parameter, not a followed alias.

| Records at `obj.callback()` | Head | Main |
|---|---|---|
| Source | `UnknownName`, no target | Same |
| IndirectResolution | **Exact free_single → other.js:otherTarget@1-3** | **Absent** |

This is an added record at a site outside the predicate. Whether the callback heuristic's candidate is appropriate in some other program does not rescue the promised record equality here. The PR makes that existing heuristic edge reachable at a site where main never filed it.

**Evidence:** [probe log](probes.log), [head records](fixtures/cross_same/head.jsonl), [main records](fixtures/cross_same/main.jsonl). The [separate-line control](fixtures/cross_separate/) moves only `obj.callback()` onto its own line: both binaries then have the callback's textual record. That rules out file-loading, module resolution, or a global change in callback extraction as the cause.

**Mechanism:** [call_graph.rs:2779](/Users/wesleyjinks/code/prism-js-alias/src/call_graph.rs:2779) removes `h()`'s Level-1 record from `extra_sites`. The later Level-4 guard at [call_graph.rs:2844](/Users/wesleyjinks/code/prism-js-alias/src/call_graph.rs:2844) considers a call already resolved if **any** extra record for the caller is on the same line. On main, `h()`'s record prevents Level 4 at the neighboring site. On head it is gone, so Level 4 adds the callback record. The suppression changes control evidence used elsewhere.

**Recommended family closure:** Compute the complete legacy indirect-record population with unchanged pass ordering and guards, then filter its output by exact source-site identity where the eligible alias predicate is true. Suppressed records must continue to participate in the legacy computation that determines records for other sites. This closes cross-site effects by construction, rather than adjusting a guard for this example. Add a same-line multi-site fixture to the structural test; its current `with.difference(without)` assertion should reject the added callback record.

**Alternative:** Maintain a separate, unsuppressed collection for all later legacy guard decisions while filtering the emitted collection. Cost: two representations and the need to audit every later dependency on `extra_sites`. Merely changing Level 4's guard to compare byte spans alters main behavior at false-predicate sites, so it does not preserve this PR's contract.

**Does not apply:** No later pass depends on the suppressed record, including the separate-line control. Non-alias callback heuristics that are identically reachable on main are outside this finding.

**Confidence: 100/100.** Exact site-span outputs and the line-separation control expose the dependency. Filtering only after legacy computation would collapse it.

## SMELL findings

**S1 — SMELL / IMMATERIAL: the live PR description still describes the round-2 artifact.** It says the stand-down is based on a followed binding, retains the now-removed span-less exception, lists 1,335 fixtures/41 mutants, and says round 2 is in progress. The head/source comments and `CLAUDE.md` have changed. Recommended fix: rewrite the [PR body](https://github.com/shoedog/prism/pull/372) around the final artifact, current checks, and the actual unresolved boundary. Alternative: a clearly linked superseding final-head report, at the cost of contradictory visible prose. Does not apply once the body is reconciled. This documentation mismatch is not an additional behavioral blocker.

## Answers and closure

**1. Is the rule true by construction? No.** The terminal outcome filter is genuinely singleton Exact for all three terminals, and false-alias resolver fallback is now locally main-equivalent. But W2 shows a true predicate rejected before the alias rung; W4 changes records at a false-predicate neighboring site; W3 credits a true alias to the wrong traversal identity. W1 separately violates the admitted binding proof.

I enumerated every production reference to the three variants: their persistence/construction, the terminal route, import proof, telemetry; and the predicate/outcome consumers in Source resolution, indirect computation, navigation incoming construction and callers formatting. Other `local_binding` matches preserve the old fallback classification for false aliases. Synthetic indirect records explicitly have `Unchecked` binding; `MacroArg` is Rust-only. No further variant-specific false-alias Source difference was found. This local equivalence is not a proof that the graph's record population or formatter is equivalent.

**2. Is the structural test sound? It is useful for its asserted subset, not a test of the entire stated rule.**

- It loads every matrix fixture, traverses every Source site, including emitted JSX sites, and has non-vacuity guards for both predicate outcomes and textual suppression.
- `MayCall("alias")` is the real main binding for a followed alias: following is entered only from `JsBinding::Alias`, and the unchanged mapping converts that answer to `MayCall("alias")`. The comparison preserves target file/name/start/end, confidence, resolution kind and drop reason. It is nevertheless head-vs-head; a shared regression in fallback logic could survive, so the supplied two-binary sweep remains separate evidence.
- The audit clears held indirect records on a clone and uses the product computation with the same export facts; product full and cached build paths both install export resolution before recomputing indirect records. The held-vs-audit-on assertion prevents the seam from silently representing a different emission path for these fixture keys. It does not mutate the original graph or expose a product stand-down switch.
- The textual projection compares sets of `(caller identity, start byte, end byte, callee spelling)`, not full serialized records or callers-bucket multiplicity. Its fixed synthetic-record factory reduces that limitation for today's seam; it is not a general full-record comparator.
- The true branch at [js_alias_rule_test.rs:103](/Users/wesleyjinks/code/prism-js-alias/tests/integration/js_alias_rule_test.rs:103) just records the site and continues. It never checks the public Source outcome. The lower-case destructure fixture in W2 can therefore pass this test even if added: true branch skipped, no textual records on either side. Callers output is also never checked there.
- The test would catch W4 **if its multi-site fixture were present**, because stand-down-on adds a record absent with stand-down-off. Neither the matrix's finite shape set nor a sweep of that same set is a construction proof for unrepresented interactions.

Local structural tally, reproducible in an isolated repeat: **1,349 fixtures, 1,825 Source sites, 348 alias-bound, 303 true, 45 false compared, 15 held indirect records, 6 suppressed**. Those Source/alias/false/held totals differ from the brief's 1,811/338/35/14; I report the actual output and do not infer a cause from it. Zero violations of the test's existing assertions.

**3. Round-2 handling rows:**

| Round-2 item | Closure on this head |
|---|---|
| Another name duplicated inside `{ f, x: a, f: a }` | Specific defect closed; four named matrix variants equal main. The broader declaration-admission family remains open in W1. |
| Callable terminal returns `WrappedExportNonJsx` instead of main | Closed for the reported wrapped-call population: all eight named matrix variants equal main. Common singleton filtering now covers callable terminals too. W2 is a separate pre-rung site gate. |
| Binding-kind stand-down removes unproven records / CJS edge | Reported examples closed: both own-name CJS matrix rows and the span-less nested-decoy row equal main; the same-function CJS row retains two records. Record-parity family remains open in W4. |
| `import_proven` leaks into fall-through | Closed: `alias_route && local == binding.local`; the paired reentrant paths/wrapped fixture equals main. |
| Formatter credits every same-named function | Added-list cross-file/nested identity checks are repaired and their tests pass. Original spelling-bucket admission remains open in W3. |
| Counter semantics, module comment, `CLAUDE.md` | Old `unresolved` outcome removed and comments/docs corrected. Eligibility overcount is demonstrated in W2; live PR body is S1. |
| Owner-decided follow-ons | Accepted scope exclusions; no findings raised. |

The 15 named matrix fixtures plus the custom import-proof fixture were independently run against both releases: **16 identical complete call-site record sets**. See [closure-probes.log](closure-probes.log) and retained sources/output under `closure/`. This confirms those repairs without transferring their closure to the broader families.

**4. Pattern admission:** No, flat entries plus unique names do not prove declaration validity. W1 names five remaining early-error declarations and the observations separating them from the repaired duplicates and escaped identifiers.

**5. Rework surface:** The clone-based audit seam is read-only and uses the product computation; its public hidden surface has no demonstrated material defect. The new Boolean is correctly passed as `true` by the product, but local suppression is not graph-wide parity (W4). Carrying identities through the formatter fixes its added list, not its original bucket (W3). Calling the route directly also exposes the missing public eligibility gate (W2).

## Probe and validation log

| Hypothesis / discriminating prediction | Observation / control |
|---|---|
| Same-line Level-4 guard depends on suppressed alias record; predicts new record at neighbor, falsified by identical neighbor records | Confirmed; separating the lines restores neighbor equality. |
| Semantic invalid declarations survive shape/uniqueness; predicts syntax error plus head Exact, falsified by head equality with main | Confirmed five forms; valid pattern follows, duplicated and escaped sibling controls equal main. |
| Formatter's original name bucket bypasses identity; predicts wrong depth-2 caller, falsified by exclusion | Confirmed; depth 1 isolates initial attribution, direct-call twin exposes inherited reachability on both binaries. |
| Intrinsic JSX route hit differs from public resolver; predicts true counter plus Source drop and formatter-only caller | Confirmed; uppercase twin resolves and both head consumers credit it. |
| Reported round-2 rows are repaired; predicts complete record equality | Confirmed all 16 paired checks. |

The initial cross-file formatter probe omitted the caller file from the diff and returned empty output even for its direct-call control. That was **inadmissible** for callers inclusion. The corrected diff includes the import line so the file is loaded, without creating a touched function there; the final comparison uses fresh cache directories. A connector workflow request with wrong argument names was inadmissible and corrected before interpreting CI. Wrong-working-directory auxiliary reads supplied no evidence; corrected source/artifact reads are the basis of this report. Exit status was not used as behavioral evidence.

Local verification:

- `cargo test --offline --lib js_binding_alias`: **19 passed, 0 failed** ([log](lib-tests.log)).
- `cargo test --offline --test integration js_alias -- --nocapture`: **82 passed, 0 failed**, including the 1,349-fixture matrix and structural test ([log](integration-tests.log)).
- Isolated structural repeat with blessing explicitly unset: **1 passed, 0 failed** ([log](rule-tests.log)); this repeats one of the 82, not another unique test.
- Total unique scoped tests run: **101 passed, 0 failed**. The worktree remained clean. No implementation repair or proposed fix was executed.

### Checked and found sound

- Singleton Exact filtering of all three terminal routes, subject to W2's site gate.
- False-alias Source fallback and the repaired route-only import proof; the 16 paired closure checks.
- Duplicate BoundNames inside one flat pattern, including another name; escaped-identifier refusal and valid-pattern positive controls.
- Export-resolution ordering before indirect recomputation in full and cached build paths, by reading.
- Audit-on product computation and held-key consistency on the matrix; audit operates on a clone.
- Exact identity filtering for the formatter's added alias list and the existing cross-file/nested tests; legacy bucket defect is W3.
- Scoped tests and current nine-job CI status.

### Not checked

- The full local test suite, doctests, MCP feature suite, or Tier-A harness; this was a read-only review with the authorized scoped tests, not implementation completion. CI job status is not independent local execution.
- The 45-mutant lane; its authoritative result is supplied evidence, not my execution.
- An independent rerun of the complete two-binary matrix sweep, corpus dumps, TypeScript checker, or ledger rescoring. The controller's aggregates are supplied evidence. No private or forbidden corpus directory was opened.
- Partial-hit cache execution, experimental executable-owner routes, or runtime module mutation.
- The proposed fixes, their regression tests, or a proof of every possible declaration early error. The recommended admission boundary needs owner/design resolution at this cap.
- Binary build provenance beyond supplied paths, displayed revisions and measured hashes; earlier CI attempts/logs.

**Verdict: FIX — 4 WRONG / MATERIAL; 1 SMELL / IMMATERIAL. Round 3/3, final cap reached; existing families remain open-class, with one additional bounded site-eligibility defect.**
