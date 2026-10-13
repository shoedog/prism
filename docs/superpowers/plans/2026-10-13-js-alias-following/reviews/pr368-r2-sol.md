# PR #368 — confirmation, round 2 of 2

**FIX — two bounded corrections to the compatibility statement remain.** The namespace authority repair is sound, and the lone `x.js.ts` objection in my round-1 report is refuted by TypeScript 5.9.3. I found no material implementation defect introduced by the fold. However, the restated contract still excludes real, sound edge changes and promises canonical parity that the fold deliberately does not preserve. Correct the statement and pin the additional namespace consequence; neither requires a fresh implementation or another review round.

## Binding and evidence

Reviewed clean `/Users/wesleyjinks/code/prism-js-i1`, branch `feat/js-relative-import-extension`, HEAD `a34a1d454ff5fc55076a3e0307c9e6c6742431ac`, base `e2701e89c2fb09cd268232b041b95f7d1ce1e7ea`, nine commits. Confirmation scope: the last five commits, `aa74eae0..HEAD`, plus their consumer seams and both original reports. GitHub read-only metadata for [PR #368](https://github.com/shoedog/prism/pull/368) agrees with both revisions. CI run `38470504010` and its nine jobs are successful at this head. No GitHub writes or Git writes were made; source remained clean.

Executed:

- `cargo test --offline --lib js_relative_candidates`: **35 passed, 0 failed**, 1,390 filtered; [unit.log](unit.log).
- `cargo test --offline --test integration js_relative`: **33 passed, 0 failed**, 1,119 filtered; [integration.log](integration.log).
- Additional selections: legacy `d5_authoritative_missing_member_and_fallback`, CPG version pin and nav version pin each **1 passed, 0 failed**; [namespace-legacy.log](namespace-legacy.log), [cache-cpg.log](cache-cpg.log), [cache-nav.log](cache-nav.log). Total selected test executions: **71 passed, 0 failed**.
- All-lane `mutgate.py --plan-only`: **247 selected, exit 0**; [plan](mutgate-plan/schemata-plan.json). All 17 I1 anchors and three advanced cache anchors occur exactly once; [anchor audit](anchor-audit.json). Plan-only executes no mutants, regardless of its printed `AUTHORITATIVE` label. The reported 17+3 kills remain supplied evidence.
- **36 synthetic layouts, 72 binary runs**, using separate fresh caches; complete `record_kind: call_site` rows retained under [outputs](outputs/). The principal results are in [probe-results.jsonl](probe-results.jsonl) and [followup-results.jsonl](followup-results.jsonl); hypotheses, alternatives and results are in [probe-log.md](probe-log.md).
- Six direct TypeScript `resolveModuleName` calls, version **5.9.3**, over two separate scratch layouts and three modes; [script](probe_ts.cjs), [results](ts-resolver.jsonl). No corpus packages were executed.

The binaries report branch `a34a1d454ff5` and frozen main `40ad8390d6cb`. The latter is older than the PR base. I rechecked its source diff against `e2701e89`: changed files are MCP code and cache/navigation provenance; the candidate, namespace, export and receiver resolver sources are unchanged. The tests use current source; the behavioral main control uses the supplied frozen binary in the same environment, not a newly built PR-base binary. Binary SHA-256 values: branch `a01c498fd7c5c8ad0ae1ffa47639bee9fcd13bdf53ce75a885da4da4759c0e70`; main `6e3c9e259967f7b6cbc94049dd5d55cac4ab326ae16e39a6ff20e474cb3d1bf2`.

## Findings — WRONG first

### W1 — WRONG / MATERIAL: clause 3 still excludes namespace target removals, including an existing Exact removed without any star conflict

**Confidence: 100/100. Contract defect; the narrower branch output is correct.** This is the remaining closure work for round-1's edge-compatibility finding, not a claim that the fold introduced a new incorrect binding.

Input [ns_reexport_canonical](fixtures/ns_reexport_canonical/):

```ts
// app.ts
import * as N from './x';
export function g() { N.f(); }
// x.ts
export { f } from './impl/x.js';
// impl/x.ts
export function f() {}
// other/x.ts
export function f() {}
```

| Same `N.f()` site | Full target set |
|---|---|
| Frozen main | `impl/x.ts:f` **Exact/import_qualified**, `other/x.ts:f` **Exact/import_qualified** |
| Head | `impl/x.ts:f` **Exact/import_qualified** |

[Main record](outputs/ns_reexport_canonical-main.jsonl), [head record](outputs/ns_reexport_canonical-branch.jsonl). The same removal occurs with outer `./x.js`, which main already found using `namespace_sibling`; [paired results](followup-results.jsonl), fixture `ns_reexport_new`.

The newly found module is the inner `./impl/x.js` hop. There is **no `export *`** and no ambiguity. Main cannot prove the outer namespace export and invokes its legacy Exact stem fallback. Head proves the re-export identity and narrows that pool. Thus an existing Exact target disappears outside the only Exact-removal kind named in clause 3.

There is a second grade in the same bounded mechanism. For `import * as N from './a/../x.js'`, where `x.ts` and `other/x.ts` both directly export `f`, main returns both **NameOnly/import_qualified**; head returns only `x.ts:f` **Exact/import_qualified**. The unrelated NameOnly edge is removed, although clause 3 permits NameOnly loss only for receivers. This is already the positive control in the new namespace test; [main](outputs/ns_positive_redundant_js-main.jsonl), [head](outputs/ns_positive_redundant_js-branch.jsonl). Calling the promotion a gain does not account for the distinct target that disappeared.

**Mechanism:** `apply_js_export_resolution` builds namespace facts with the expanded hop resolver (`src/call_graph.rs:2291–2303`); `namespace_identity` follows named re-exports (`src/js_exports.rs:321–338`); `js_ts_namespace_proof` filters the base pool to one file/name/span identity (`src/resolution_js_namespace.rs:56–81`). The legacy fallback at `src/resolution.rs:2617–2620` can grade its entire pool Exact. The new early-return guard is not responsible for this removal: `found_on_main` is true for the outer module and positive proof is returned in either case.

**Recommended fix:** extend clause 3 to permit removal of speculative namespace targets, **NameOnly or legacy Exact**, when a newly resolved module/hop enables positive namespace proof. Preserve the current output. Add an integration row for the named-hop fixture above, asserting the entire remaining target/grade/kind set; include the canonical inner `./impl/x` twin, which already narrows on main. The new direct-positive namespace test already pins the NameOnly case. Update the PR description and any handoff repeating the three-kind enumeration.

**Alternative:** keep a legacy namespace export projection and preserve its old pool whenever a newly admitted hop changes positive proof. Cost: a second projection/provenance policy, retained false Exact edges, and loss of useful precision. It defeats the stated aim of making new spellings work consistently.

**Does not apply** when the pool contains only the proved identity, or the expanded hop still supplies no positive proof. Newly found outer modules with failed proofs retain NameOnly under the fold; main-found modules with failed proofs retain the legacy fallback. Those cases were tested. The finding collapses once the contract explicitly permits this proof-driven pruning and the named-hop row is pinned; no product change is requested.

### W2 — WRONG / MATERIAL: clause 2 promises universal canonical parity despite three explicit tier-policy exceptions in the fold

**Confidence: 100/100. Contract defect; these are deliberate source-policy differences, not a request to undo the accepted tier repair.** The fold itself acknowledges these exceptions in `redundant_segments_follow_typescript_order_where_canonical_is_legacy`, `src/call_graph/js_relative_candidates_test.rs`.

Every file below exports `f`; caller `app.ts` imports `f` and calls it in `g`. Compare redundant `./a//x.js` with canonical `./a/x.js`:

| Indexed files | Main, redundant | Head, redundant | Main **and** head, canonical |
|---|---|---|---|
| `a/x.js.ts`, `a/x.ts` | drop `UnknownName` | `a/x.ts:f` Exact/import_member | `a/x.js.ts:f` Exact/import_member |
| `a/x.ts`, `a/x.js/index.ts` | drop `UnknownName` | `a/x.ts:f` Exact/import_member | `a/x.js/index.ts:f` Exact/import_member |
| `a/x.js`, `a/x.js.ts` | drop `UnknownName` | `a/x.js:f` Exact/import_member | `a/x.js.ts:f` Exact/import_member |

Evidence: `suffix_competing_*`, `suffix_directory_*`, `suffix_literal_append_*` in [results](probe-results.jsonl), with complete raw records under [outputs](outputs/). Each is a changed target identity across spellings, not an assertion-format or missing-export issue.

**Mechanism:** the literal guard returns the old canonical list unchanged (`src/call_graph.rs:6711–6717`), whereas the newly normalized spelling selects literal/substitution/appended tiers (`:6764–6792`). Main's canonical list has older precedence and cardinality rules. Both can coexist, but they do not establish the blanket spelling invariant in clause 2.

Reading “canonical” as the extensionless source spelling does not make parity universal either: with `a/x.ts` exporting class `C`, and `a/x.js/index.ts` exporting only `other`, `./a//x.js` yields two candidates and `C.m()` stays **NameOnly/r6_single_owner** on main and head; `./a/x` yields one and is **Exact/typed_param** on both. [Class controls](followup-results.jsonl), fixtures `class_index_*`. This is a cardinality/recall limitation, not an additional changed-edge finding.

**Recommended fix:** qualify clause 2 with the retained literal-list precedence/cardinality exceptions, and scope it to the modeled consumer forms. State the actual rule: existing lists are preserved; newly admitted spellings use the normalized tier policy; canonical parity holds where those policies select the same candidate list and the consumer admits the form. Retain the existing four-row candidate test and add a complete import-member twin assertion for these three differing target selections if the PR continues to make an edge-level parity claim. Bare-dot namespace/default-qualifier/import-forward and paths-hop exclusions already recorded after round 1 must be explicit exceptions to the universal sentence, not merely a separate follow-up list.

**Alternative:** restore universal parity by resolving the normalized literal spelling through exactly the legacy list before considering new tiers. Cost: reintroduces `x.js.ts` beating available `x.ts` in the redundant spelling and the other legacy orderings that the fold deliberately corrected. Updating both canonical and redundant lists instead would violate clause 1 and broaden the slice.

**Does not apply** in ordinary single-source cases, where the candidate lists agree; the parity and consumer tests cover those. It also ceases to be a contract defect once the policy exceptions are stated. No new implementation defect is demonstrated by these intentional tier differences.

## Closure of every round-1 handling row

| Handling row | Confirmation |
|---|---|
| Newly found namespace module lacks member: sol W1 / Opus W3 | **Closed.** Missing value export, type-only export and ordinary-call wrapped proof failure preserve main's complete NameOnly pool for redundant `.js` and extensionless spellings. Positive proof still selects only the real identity. Plain `./x.js`, `./x`, and literal-plus-sibling cases preserve legacy outcomes. |
| Explicit `.js` appended-file precedence: sol W2 | **Closed as an implementation finding; original lone-file premise refuted.** Head selects `x.ts` before `x.js.ts` for the redundant spelling when both exist. A lone `x.js.ts` remains admitted and TypeScript 5.9.3 confirms it. Canonical parity wording remains W2 above. |
| `.d` stem reaches `.d.ts`: Opus S5 | **Closed.** The shared fallback refuses a stem ending in `.d`; complete unit rows and binary declaration-only controls stay empty. The unchanged namespace sibling can still find declarations as on main; it is not the new substitution lane. |
| Newly resolved star exposes conflict: sol W3 / Opus S1 | **The named star case is closed.** Conflict drops, same-declaration and absent-export controls retain Exact; tests and same-environment binaries agree. **The broader compatibility reconciliation remains open as W1**, because namespace proof also removes old targets. |
| Props inherited method: Opus W2 | **Closed by the accepted policy, not repaired.** Main `.js`: NameOnly/r6_single_owner; head `.js`: ExternalReceiver; canonical `./props` drops on both. The twin test asserts the complete drop result. No inheritance implementation was promised. |
| Three stale mutation anchors: Opus W1 | **Closed.** Advanced anchors each occur once, all-lane plan exits 0; pins checked. Reported three kills are supplied evidence, not rerun. |
| Survivors / untested consumers: sol S1 / Opus S4 | **Closed for the enumerated requests.** Terminal `/.` and `/..` with same-stem distractors, trailing `.js/` substitution refusal, trim, all six index extensions, typed parameter, constructor local and two-candidate refusal are present and pass. The two-candidate receiver test uses twin equality plus “no Exact,” rather than fixing its entire fallback row; that is an **IMMATERIAL SMELL**, not a demonstrated wrong result or blocker. Tightening it to an explicit full expected row would protect against synchronized fallback drift; the alternative is keeping twin equality plus a source-bound full-row mutation check, which costs more gate maintenance. It is unnecessary if the test intentionally promises only parity and Exact refusal. |
| Overbroad CLAUDE.md sharing: sol S3 / Opus S5 | **Closed.** Sharing is scoped to import-member, re-export and namespace; paths and forwarding boundaries and the failed-proof namespace exception are named (`CLAUDE.md:212–218`). |
| Mixed units / stale adjudication: Opus S5 | **Closed at the recorded-evidence level.** Live PR separates site keys and targets. Current X adjudication has seven `agree`; T's categories total 15,419, including 2,775 overload-implementation agreements. I read these aggregates, not the checker runs themselves. |
| Follow-ups, explicitly excluded | **Parked as authorized, not fixed or reopened.** Bare-dot namespace/default-qualifier/import-forward, paths `.js` hops, `.jsx` sibling asymmetry, and canonical namespace stem fallback remain outside this slice. `package.json main` remains the prior **WRONG / IMMATERIAL** model limit; disclosure satisfies its accepted handling, not runtime Node fidelity. These exclusions also require the clause-2 qualification described above. |

## Fold code and disputed point

**“Found on main” is computed correctly for the supported indexed paths.** Main's lookup was first literal candidates, then `namespace_sibling`. The predicate is exactly nonempty literal-list OR sibling success, using the unchanged literal helper for both checks. Bare dots have neither; redundant internal segments cannot acquire a sibling through the new normalizer; ordinary `.js` already finding `.ts`/`.tsx` through the sibling is retained. A module found by both still chooses the literal answer first. Returning to the literal helper inside `namespace_sibling` is essential: using the expanded helper there would incorrectly classify normalized modules as old.

For a main-found module, `found_on_main` makes the new early-return condition true, so the extracted proof returns the same `Some`/`None` policy as before. The extraction preserves file/name/span, wrapper/JSX and exactly-one-base-identity checks. **This proves the guard does not change legacy failed-proof policy; it does not prove the positive export facts cannot change.** W1 supplies that distinction.

**Tiering matches the accepted policy.** Literal first, `.js` stem `.ts` then `.tsx` with the dotfile and `.d` guards, appended-extension tier, then directory indexes; directory-only and root-escape guards remain intact. A nonempty literal list returns before tiering, so no tier can change that list. The three canonical exceptions are intentional and tested, but must qualify the parity claim. Literal-JS-first is prism's compatibility policy; it is not full TypeScript precedence when a TS source and JS file coexist, which the PR itself discloses.

**I accept the lone `x.js.ts` behavior.** My earlier statement that TypeScript could never select it was wrong. The direct six-call probe returns `x.js.ts` alone in Bundler, Node10 and NodeNext, and `x.ts` when both are present. Separate directories, no supplied cache and explicit options rule out the proposed cache/configuration alternative. This is a mechanism-level refutation of the inherited WRONG premise, not a downgrade based on failing to find a counterexample. The competing-file precedence portion is fixed.

## Checked and found sound

- Exact checkout/base/fold binding; clean source; both binary identities and same-environment controls.
- Literal candidate-list preservation, supported path normalization, root escape refusal, directory-only guards, suffix/stem guards and all six index probes.
- Namespace main-found classification, proof extraction, legacy sibling/literal/both precedence, new failed-proof NameOnly preservation and positive identity narrowing.
- Accepted star-conflict and inherited-props consequences, with separating controls.
- Enumerated new tests, mutation anchors and current cache pins; documentation's narrowed sharing sentence.
- Direct TypeScript 5.9.3 resolution of the disputed lone and competing appended-file cases.
- Live PR units, nine successful CI jobs, and current public-corpus adjudication aggregates.

## Not checked

- Full suite, clippy, fmt or Tier-A locally; this was a bounded read-only confirmation, not implementation. Successful CI jobs were read. The live PR reports a local full-suite run with 5,414 passed, six failures and one skipped; its five MCP timing failures lack a repeated control at this head, so I do not independently attribute them to environment or regression.
- Authoritative mutation executions; only anchor admissibility and selected test behavior were rerun.
- X/T corpus measurements, their byte-identical before/after-fold diffs, the 1,159-fixture matrix and checker declaration comparisons; supplied measurements remain source-bound claims, not my executions. No private corpus or excluded directories were inspected.
- Executable-owner consumer end to end, incremental cache partial-hit behavior, symlink/case-insensitive path behavior, and package.json directory resolution beyond its accepted follow-up boundary.
- Universal semantics over all JS/TS programs. “No new material implementation defect found” is the result of this confirmation scope, not an exhaustive language proof.

At the declared cap, these are closed, enumerable contract/test corrections. The original implementation findings have converged; there is no open-class implementation population and no justification for a restart or an undeclared third round.

**Verdict: FIX — qualify clauses 2 and 3 and pin named-re-export namespace pruning; no new review round or product restart.**
