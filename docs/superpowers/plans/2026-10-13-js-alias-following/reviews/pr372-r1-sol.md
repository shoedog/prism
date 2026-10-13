# Slice L review — round 1 of at most 2

**Verdict: FIX.** One new false Exact claim, with a bounded repair. No new wrong target was demonstrated on a semantically valid program. The blocker is an early-error declaration that the positive follower incorrectly treats as a unique binding. Three nonblocking SMELLs are listed separately.

## Review binding and execution

Reviewed the clean worktree `/Users/wesleyjinks/code/prism-js-alias` at `3dd5b7db53f389603e26d29a8bc9708d241f0b4d`, using `origin/main...HEAD`. The measured merge base is `38a5999487716a01ef6e0112a2b676e802909c1d`. The matrix baseline at that base equals the one at `origin/main` (`86756aadeec8baf5a3185bfdc38de10256d2ebfa`).

Synthetic comparisons used the supplied prebuilt binaries, with separate fresh scratch caches for each fixture and binary. Binary revision labels are inherited from the brief; I did not rebuild or independently establish their build provenance. SHA-256:

- L2-prism: `d1415902f591238773c8cad99c9c152ace0895af78b3e7e74d6e43d4e229476e`
- main-38a59994-prism: `9083763933fd7f4128e50541542c012849d07d4d0254ffe611b7d87102b14227`

The harnesses `probe.py` through `probe5.py`, fixture sources under `fixtures/`, and paired `<case>-head.jsonl` / `<case>-main.jsonl` outputs are alongside this report. Every comparison used `nav --cache-dir <fresh scratch directory> call-stats --repo <fixture> --dump-sites`, inspecting `record_kind: call_site` records, not merely exit status. No worktree source changes, Git writes, GitHub actions, forbidden directory access, corpus execution, or release rebuild occurred.

## Findings — WRONG first

### W1 — WRONG / MATERIAL: forbidden sibling patterns bypass duplicate-binding refusal

**Input:** `fixtures/duplicate-rest-pattern/app.js`:

```js
import * as N from './callee';
export function run() {
 const {f, ...f} = N;
 return f();
}
```

`callee.js` contains `export function f() {}`. The two occurrences bind the same lexical name. This is an early SyntaxError, so the site has no valid, uniquely declared callable binding. Head emits an **Exact import_member** edge to `callee.js:f@1-1` at line 4; main emits one Source record with **UnknownName**, no target. See `duplicate-rest-pattern-head.jsonl` and `duplicate-rest-pattern-main.jsonl`.

The complete demonstrated population uses the same mechanism:

| Pattern | Site | Head | Main |
|---|---|---|---|
| `{f, ...f}` | `f()` | Exact `callee.js:f` | UnknownName |
| `{f, x: {f}}` | `f()` | Exact `callee.ts:f` | UnknownName |
| `{f: a, ['x']: a}` | `a()` | Exact `callee.js:f` | UnknownName |
| `{f: a, 'x': a}` | `a()` | Exact `callee.js:f` | UnknownName |

The respective cases are `duplicate-rest-pattern`, `duplicate-nested-pattern`, `duplicate-computed-pattern`, and `duplicate-string-pattern`. `node --check` on scratch `.mjs` copies reports `SyntaxError: Identifier 'f'/'a' has already been declared`; it does not execute the modules. The last three JS checks are preserved in `*-syntax.txt`; the nested-pattern diagnostic was also observed this turn.

**Mechanism:** [js_binding_alias.rs:156](/Users/wesleyjinks/code/prism-js-alias/src/ast/js_binding_alias.rs:156) checks defaults throughout the pattern, but its entry loop only collects matching admissible entries and ignores forbidden siblings. Thus the shorthand/rename yields one key even when a nested/rest/computed/string entry binds the same name. [js_binding_decls.rs:85](/Users/wesleyjinks/code/prism-js-alias/src/ast/js_binding_decls.rs:85) collects bound names into a `BTreeSet`, so multiplicity within one declarator disappears before the usual duplicate-declaration guard. This is different from a parse-recovery failure: tree-sitter admits the grammar; the missing check concerns a semantic early error. It is also different from ordinary valid rest siblings, which can bind the selected member correctly.

**Recommended fix:** Validate the entire object pattern before selecting a link: apart from comments, require every entry to be shorthand or an ordinary property-identifier key with an identifier value. Reject nesting, rest, computed/string/numeric keys and defaults anywhere. This matches the supplied contract and closes all four demonstrated variants without changing the general binder. Add unit negatives and paired integration/matrix regressions. The current positive “pattern with a rest sibling” test at [js_binding_alias_tests.rs:140](/Users/wesleyjinks/code/prism-js-alias/src/ast/js_binding_alias_tests.rs:140) must be reconciled with the stated “no rest” contract.

**Alternative:** Deliberately support valid forbidden siblings, but count every BoundName occurrence without deduplication and reject duplicate names before following any entry. Cost: a broader pattern contract and additional review/testing, including nested arrays and rest. Simply counting matched keys is insufficient.

**When this does not apply:** Fully admissible patterns without duplicate bound names; a contract explicitly restricted to semantically valid inputs would remove the early-error counterexample, but that restriction is absent from this brief, which explicitly asks about duplicates and recovery. Valid `{f, ...rest}` alone is a contract discrepancy, not evidence of a wrong target.

**Confidence: 100/100** for the measured new Exact overclaim on these early-error inputs. An occurrence-sensitive refusal would collapse the finding; different evidence on valid programs would not erase these paired results.

### S1 — SMELL / MATERIAL: the matrix omits a correct edge intentionally lost by the drop rule

`fixtures/lost-export-outer-id/producer.ts`:

```ts
export function h() { return 1; }
export const x = h;
```

`app.ts`:

```ts
import { x } from './producer';
export function outer() {
 const h = x;
 return function run() {
  return h();
 };
}
```

Main emits **Exact free_single producer.ts:h@1-1** at line 5. Head drops **UnknownName**. The syntax independently establishes that the captured `h` holds `producer.ts:h`. `lost-export-outer-member` reproduces the loss with `import * as N` and `const h = N.x`. The module-level versions drop on both binaries because main's module-binding guard prevents its repo-wide rung; moving the alias to the outer function is the separating observation.

This loss is **required by the chosen contract**: the binder reaches an import/member terminal, the export summary deliberately leaves `x` unproven, and the resolver returns rather than trying name rungs ([resolution_js_alias.rs:24](/Users/wesleyjinks/code/prism-js-alias/src/resolution_js_alias.rs:24), [resolution_js_alias.rs:80](/Users/wesleyjinks/code/prism-js-alias/src/resolution_js_alias.rs:80)). It is not a request to restore spelling-based Exact authority, and not a blocker against implementation of that contract. It does refute a general “this positive slice never loses correct edges” interpretation; the supplied corpus/matrix measurements remain valid within their denominators.

**Recommendation:** Add these ground-truth fixtures and explicitly record the accepted CORRECT→MISSING transition and its fail-closed rationale. **Alternative:** Extend export summaries to follow exported aliases and preserve the edge through proof; that is a larger, explicitly deferred cross-file slice. **Does not apply:** A proven export terminal, or a base site already dropped/demoted. Confidence in the observed loss: 100/100; its acceptability is the owner's contract choice.

### S2 — SMELL / IMMATERIAL: the new docs sentence describes the next slice's refusal policy

[CLAUDE.md:246](/Users/wesleyjinks/code/prism-js-alias/CLAUDE.md:246) says a local alias resolves Exact “only” through the const chain and that spelling is not binding evidence. As a statement of current output, this is inaccurate. `fixtures/doc-let` has `let h = f; h()`; **both** binaries emit an Exact import-member `IndirectResolution` record targeting the imported `f`, plus a dropped Source record for `h`. The unchanged by-name paths can likewise still emit Exact for unfollowed aliases. This old edge is not a product finding against this PR; the newly introduced unqualified documentation assertion is the concern.

**Recommendation:** Describe the new positive route and explicitly say unfollowed aliases retain legacy resolution pending the next slice. **Alternative:** Label the sentence as the intended final policy, with current exceptions. Cost: only wording. **Does not apply:** After the refusal slice eliminates those paths, or when the sentence is explicitly normative rather than a description of shipped behavior. No new incorrect product behavior is alleged.

### S3 — SMELL / MATERIAL: end-span equality is not independently exercised by the mutation lane

A behavior-changing candidate survivor is replacing `fid.start_line == s && fid.end_line == e` with `fid.start_line == s` in [resolution_js_alias.rs:101](/Users/wesleyjinks/code/prism-js-alias/src/resolution_js_alias.rs:101). Existing AF-31 removes the whole span filter; its nested function starts on a different line, so it does not independently protect the end-line conjunct.

`fixtures/end-span-conjunct/callee.ts` contains:

```ts
export function f() { function f() {}
 return 1;
}
```

With `const h = N.f; h()` in `app.ts`, head correctly resolves to outer `f@1-3`; main drops. A start-only mutant would also admit inner `f@1-1`, yielding two candidates and dropping the correct edge. The selected follow tests and matrix lack this same-start/different-end collision (the matrix scan found no same-line pair of `function f` declarations). This is a **predicted**, not executed, mutant survivor; no source mutation or mutant rebuild was authorized/performed.

**Recommendation:** Pin the full target identity for this fixture and add the end-only mutant. **Alternative:** Use declaration byte identities throughout resolution, at greater implementation/cache cost. **Does not apply:** A test already selecting two same-name declarations with the same start and differing ends, or a function index guaranteeing that situation impossible; the measured fixture shows the current index can represent it. This is test hardening, not a demonstrated implementation error.

## Answers to the review questions

1. **Wrong edges:** W1 is the new overclaim found. The alias and RHS walk reuse the existing nearest-scope and write proof; 88 binder tests cover parameters/default environments, catches, blocks, class/static bodies, for/switch scopes, namespaces, Annex B, duplicate declarations, escaped names, writes and recovery. Additional paired escaped-shadow and named-function-expression fixtures did not produce a new wrong edge. I did not prove absence over all scope combinations. TDZ/order is not newly enforced: the binder is declaration-based and does not establish execution-order safety; an Exact edge is not a proof that initialization or execution occurs. No TDZ-based runtime objection is used as a blocker here.
2. **Lost edges:** Yes, S1 supplies the provably right base edge. This is compatible with the explicit unresolvable-terminal drop policy, but absent from the current matrix's CORRECT population.
3. **Decision 1:** Keep the per-export route. In `decision1-stem`, `x.ts` exports only `other`, while `other/x.ts` exports `f`. The alias route drops `h = N.f; h()`; mirroring the current qualified route would emit Exact to unrelated `other/x.ts:f`, as the paired `N.f()` record does on both binaries. In `decision1-incomplete`, `x.ts` exports `f` plus unproven `renamed = f`: head's alias route selects only `x.ts:f`, while the qualified route emits Exact to both files' `f`. The difference is justified; do not extend the qualified-route defect.

   **Decision 2:** The stand-down is correctly scoped to the three new alias variants. Call/Return consumers use `resolve_call_site`; navigation resolves the retained Source sites. The executed integration test asserts one CPG Call edge and one `resolved_caller_edges` row for import and local callable terminals. Step 5b ([cpg/build.rs:1624](/Users/wesleyjinks/code/prism-js-alias/src/cpg/build.rs:1624)) obtains arguments from the retained Source byte span and effective spelling. The old synthetic record substitutes the terminal spelling and clears `source_callee_name`; the argument index keys by `(start_byte, spelling)` ([ast.rs:11829](/Users/wesleyjinks/code/prism-js-alias/src/ast.rs:11829)). Standing down therefore preserves the source lookup and does not remove a distinct argument occurrence. No consumer-specific origin requirement was found. The new test's name overstates its coverage: it does not assert DataFlow argument edges. That part is source-reviewed, not independently exercised by a new alias-specific DataFlow assertion.
4. **Bound, visited set, re-entrancy:** Sound. Each Alias iteration spends one link before continuing, and the check prevents a ninth. Neither classification nor React provenance re-enters the follower; wrapper provenance walks declarations without classifying its callee. Namespace proof similarly does not call the follower. Eight is a resource/policy bound, not a claim that longer chains are invalid. The 8/9 tests pin the intended limit. Cycles terminate even without `visited` because bounded, deterministic transitions cannot later escape a repeated state to a terminal.
5. **Persisted state:** The appended serde variants are covered by the CPG version 116 bump. The CPG test compares complete Debug representations of all CallSites, including the three payloads, and forces 115 to miss. Nav version 64 invalidates old resolved topology; generic round-trip and metadata-mismatch tests passed. I found no additional persisted cache version requiring a bump. There is no new alias-specific nav sidecar round-trip assertion, but the generic derived-edge serialization and version gate cover the mechanism.
6. **Tests/mutants:** The principal shapes, terminals, refusal guards, routing and stand-down are covered, but the W1 sibling exclusion is not implemented/tested as specified, S1 is absent, and S3 names a missing independent conjunct test. The JSON lane contains 34 mutants; the supplied all-killed result is inherited, not re-run. I agree the three excluded changes are equivalent for output under the remaining guards: visited-set removal leaves the finite cycle bound; removing the lexical-parent check still requires a variable declarator whose parent's first token is `const` in the pinned grammar; `(_, Some(module))` is equivalent because namespace proof only populates `module` in its Import/program/declaration-identity branch. These are mechanism arguments, not failed searches for counterexamples. Removing `at = next` is not equivalent, but the matrix's `rhs-shadowed-at-site` rows already exercise it.
7. **Matrix:** Recounted 1,169 rows, 307 changes: 239 MISSING→CORRECT, 39 WRONG→CORRECT, 2 WRONG→MISSING, 25 CORRECT field-only, 2 NONE-OK field-only. The ledger test passed with no stale entries. Sampled changes across these groups, including import/namespace/local terminals, outer-scope decoys, exported aliases, changed kinds, reasons and origins; no sampled changed row was wrong. The complete pinned test passed. Unchanged namespace-through-alias, require-result, let/var and unsupported export-alias cases remain within the declared exclusions. I found missing **fixtures**, not a demonstrated wrongly unchanged existing pinned row: W1 and S1.
8. **Docs:** S2 explains the necessary qualification.

**Out-of-scope follow-ups:** The scope-blind textual false Exact and qualified namespace stem fallback remain on main and head; do not count their unchanged direct sites as findings against this PR.

## Checked and found sound

- Exact head/base/diff binding and final clean worktree status.
- Identifier/namespace terminals, alias-site routing, wrapper JSX restriction, bound/cycle mechanism, and ordinary binder refusal/write behavior, subject to W1.
- The two implementation decisions, with the S1 drop-policy tradeoff made explicit.
- Cache version gates and serialization mechanisms.
- Matrix totals, guard/ledger, and sampled changed-row identities.
- Commands executed with `--offline`, scratch `TMPDIR`, and the bless variable **unset** for integration:
  - `cargo test --lib js_binding_alias`: 15 passed, 0 failed.
  - `cargo test --test integration js_alias`: 52 passed, 0 failed (includes the complete matrix).
  - `cargo test --lib js_binding`: 88 passed, 0 failed (includes the preceding 15).
  - `cargo test --lib followed_js_alias_bindings_round_trip_and_the_previous_version_misses`: 1 passed, 0 failed.
  - `cargo test --lib navigation::call_edge_cache::tests`: 16 passed, 0 failed.
- 157 distinct selected tests passed. Integration emitted two unrelated unused-code warnings; no regression attribution is made for those warnings. An initial relative scratch-directory creation was denied in the worktree; it was corrected to the absolute scratch path and created no worktree file.

## Not checked

- Full project suite, release build, Tier-A harness, remote PR/CI logs, and the supplied corpus aggregates. This is a bounded review, not an implementation completion claim; CI and corpus numbers are inherited from the brief.
- Independent binary build provenance, all 307 changed rows individually, exhaustive semantic scope/input space, live runtime behavior, or provider/LSP/compiler corpus checks.
- Running mutants or the proposed S3 mutant; no mutated worktree or rebuilt synthetic SUT.
- An alias-specific DataFlow argument-edge executable assertion or alias-specific nav sidecar round-trip fixture.
- The stacked refusal slice; its authorization and correctness are outside this review.

**Before merge:** Fix W1's whole-pattern admission and add regressions for the four sibling classes. Preserve the existing artifact; this finding is closed and enumerable. S1–S3 are nonblocking recommendations, not grounds to silently expand the slice.

FIX — round 1/2: reject forbidden sibling patterns before alias following; pin duplicate-bound-name negatives.
