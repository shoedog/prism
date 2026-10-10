# Lane PKG spec review — round 1 of 2

**Recommendation: FIX.** Five resolver defects produce new false Exact edges against the pinned TS 5.9.3 oracle. Another finding demonstrates false unresolved results for null exports; the dispatch instructions also point at an artifact that no longer contains the prototype. These are closed, enumerable fixes. Preserve the committed prototype and repair it in place. S2 remains parked; this review does not grant adoption or require restarting the artifact.

## Review binding and evidence

- Working checkout: `/Users/wesleyjinks/code/prism-s2-review-sol`, clean branch `review-pkg`, HEAD `c89bc5b74df05e1f8746792c0cfad2805336b0f9`.
- Base: `origin/main`, `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`.
- Reviewed prototype: `origin/proto/workspace-package-resolution`, `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318`, direct child of that base. Production/test line references below refer to this commit, read with `git show`, not the plan checkout's base source.
- Read SPEC including §0, IMPLEMENTOR, MEASUREMENTS, CENSUS, PROBES, OQ, VERIFICATION, HANDOFF, the build manifest, prototype diff, integration tests, probe sources, and relevant inherited snapshot/export/cache integration.
- All **547 production inputs and 447 test inputs** in BUILD-MANIFEST.json hash-match the committed prototype. Rehashed the retained base/head executables and facts helpers; all four match the manifest. Oracle version and SHA256 match `5.9.3` / `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
- Ran **30 small retained synthetic cases**, with actual TS ProjectService ownership and usage mode, plus both base/head facts helpers and `nav --no-cache call-stats --dump-sites` in the same environment. All 30 native predictions matched. Eight witness cases expose the five false Exact mechanisms below; every corresponding base control has no module proof and no target. No lost base edge was observed in these probes.
- Durable review evidence: [binding.json](spec-r1-sol-evidence/binding.json), [results.json](spec-r1-sol-evidence/results.json), [results-extra.json](spec-r1-sol-evidence/results-extra.json), [results-boundaries.json](spec-r1-sol-evidence/results-boundaries.json), [results-imports.json](spec-r1-sol-evidence/results-imports.json). Scripts and complete fixtures/base/head outputs are retained under `spec-r1-sol-evidence/` (under 200 KB, including report snapshot). No build, Git write, network, or private-F access occurred; the checkout remains clean and no target directory was created.

The findings grade static binding, as CLAUDE.md requires. The barrel finding below concerns an unresolved static re-export, without invoking runtime mutation.

## WRONG findings

### W1 — Explicit TypeScript exports incorrectly substitute another TypeScript extension

**WRONG · MATERIAL · confidence 100/100.**

**Evidence, reproduced:** `src/js_packages.rs:136-144`, `:239`. Fixture `exports-ts-substitution` has a linked `@ws/lib`, bundler/esnext options, `exports: "./index.ts"`, and only `packages/lib/index.tsx`, containing `export function real(){return 1;}`. The writer is:

```ts
import {real as picked} from '@ws/lib';
export function run(){picked();}
```

TS resolves no module and no callable. Head records `packages/lib/index.tsx` and grants an Exact `import_member` edge to its `real`. Base records neither. With `exports: {"types":"./index.ts","default":"./other.ts"}` and existing `index.tsx`/`other.ts`, TS instead selects `other.ts`, while head again selects `index.tsx`.

The oracle's `loadFileNameFromPackageJsonField` (`typescript.js:45450` region) directly probes explicit supported TS/declaration extensions. An exports leaf does not retry `.tsx` after a missing explicit `.ts`. The prototype's `exact` argument affects extensionless targets but does not prevent this substitution.

**Fix:** distinguish exports/package-field probing from ordinary file substitution. Explicit `.ts`, `.tsx`, and declaration exports must probe the exact file; a proven missing file returns absence so a later eligible condition can run. Preserve valid `.js` → `.ts`/`.tsx`/`.d.ts` substitution. Add both failing witnesses, plus explicit-TS-existing and declaration-winner controls. An alternative is a narrow refusal for unsupported explicit-extension situations; it prevents false bindings but must remain unsupported for S2, not native absence.

**Self-critique:** this does not apply when the exported `.ts` file exists, or to normal `.js` substitution. `extension-js-control` reproduced correct `.js` → `.ts` behavior. The paired native declarations/owner, exact head row, and empty same-environment base row eliminate wrong project selection and a pre-existing edge as explanations.

### W2 — Literal export targets have their stars erased

**WRONG · MATERIAL · confidence 100/100.**

**Evidence, reproduced:** `src/js_packages.rs:220`, `:237`, `:289`, `:321`. Fixture `exports-literal-star` uses bundler/esnext, `exports: "./index*.ts"`, and an existing `index.ts`; no file named `index*.ts` exists. TS resolves no module. Head removes the star using the empty capture, resolves `index.ts`, and grants Exact `real`. Base stays unresolved.

TS replaces stars only for a selected pattern key (`typescript.js:460xx-461xx`, `pattern ? ...replace(...) : ...`). This implementation substitutes for root/literal entries too.

**Fix:** pass an explicit pattern-selection flag through target recursion, and substitute only for pattern keys. Do not derive that flag from whether the capture is nonempty: a pattern can capture an empty string. Test root and literal-subpath targets containing `*`, plus existing wildcard-positive controls. Alternatively decline literal targets containing stars; that is sound but can miss an actual literal-star filename that TS resolves.

**Self-critique:** ordinary pattern keys such as `"./feature/*"` legitimately substitute; this finding does not dispute the tested longest-prefix ordering. Source-key order or an incorrect import/require condition cannot explain the root string fixture, which contains no conditions.

### W3 — ESM legacy main probing admits extensionless source TS rejects

**WRONG · MATERIAL · confidence 100/100.**

**Evidence, reproduced:** `src/js_packages.rs:148-151`, `:201-211`, `:415-424`. In `node16-esm-main-extensionless`, the writer's root manifest has `"type":"module"`, compiler options use node16/node16, and linked library metadata is:

```json
{"name":"@ws/lib","type":"module","main":"feature"}
```

Both `feature.ts` and `index.ts` export `real`. TS selects `index.ts`; head selects `feature.ts` and grants Exact to the wrong definition. In `node16-esm-main-missing`, removing `index.ts` makes TS unresolved while head still grants the same false Exact. Both base controls are empty.

The prototype probes `.ts`/`.tsx` for `main` without the native ESM restrictions. TS's directory loader preserves ESM mode for a `type:module` package and rejects this extensionless main; root-package lookup can then use its explicit `index.js` substitution fallback (`typescript.js:45745-45813`, `:46389`). Import-condition selection and implicit-extension permission are different facts.

**Fix:** carry actual resolution features into legacy entry probing, including the target package's captured `type` where TS uses it. Apply native main-field file/directory semantics and root fallback; do not use the condition-selection `esm` boolean as the entire loading policy. Test both false-edge witnesses and the native fallback. A narrower alternative is to refuse this specific ESM extensionless-entry class pending full support; it eliminates false positives while preserving an explicit unsupported outcome.

**Self-critique:** Node10 and Node16 CommonJS legitimately choose `feature.ts`; both controls reproduced correct head/native agreement. Explicit `main: "feature.js"` can also legitimately substitute to TS. Those distinctions rule out a universal extensionless-entry ban as the recommended fix.

### W4 — New modern package proofs expose a Node10-only relative export hop

**WRONG · MATERIAL · confidence 100/100.**

**Evidence, reproduced:** `src/js_paths.rs:470-471`, `:503-509`, `:518-566`, reached through the new package integration at `:336-337`/`:473-474`; SPEC.md:8, :21.

Fixtures `node16-esm-barrel` and `nodenext-esm-barrel` use matching modern module/resolution options, ESM writer/package manifests, and library `exports: "./index.ts"`:

```ts
// packages/lib/index.ts
export {real} from './impl';
// packages/lib/impl.ts
export function real(){return 1;}
```

TS correctly resolves the package to `index.ts`, but the writer's imported symbol has **no callable declaration** because the ESM relative re-export cannot resolve. Head nevertheless grants Exact to `impl.ts:real`. Base has no target. With bundler/esnext, the same extensionless barrel is valid and the positive control agrees with TS.

`hop` passes only `allow_js` to the inherited relative resolver; `prove_path` supplies implicit extensions/index lookup without modern ESM usage mode. PKG creates a new reachable path into that inherited helper. This is a prototype regression despite the helper itself being unchanged.

**Fix:** carry the caller project's effective options through every relative export hop and derive the barrel usage's mode from captured file/package facts. In Node16/Next ESM, reject extensionless file/directory guesses while preserving supported explicit-extension substitution. Keep caller-project ownership; do not reselect the barrel's tsconfig. Add ESM negative and bundler/CJS positive witnesses. A bounded alternative is to decline modern relative hops until modeled, at the cost of valid modern barrel yield; those refusals need the S2 coverage gate below.

**Self-critique:** the package-entry proof itself is correct here; the false result is the callable edge. This does not apply to the reproduced bundler control. Matching ProjectService owner, resolved package entry, missing native declarations, and the differing bundler control distinguish a static hop failure from unrelated wrapper or runtime behavior.

### W5 — Package-import specifiers are accepted by the ordinary package rung

**WRONG · MATERIAL · confidence 100/100.**

**Evidence, reproduced:** `src/js_packages.rs:26-35`, `:331-338`, `:374-384`, `:467-481`. Fixture `package-imports-shadow` uses bundler/esnext and root metadata `"imports":{"#alias":"./packages/lib/other.ts"}`. The writer imports/calls `real` from `#alias`. A physical `node_modules/#alias` link points to `packages/lib`, whose exports select `index.ts`; both index.ts and other.ts export `real`. TS selects **other.ts through imports**. Head classifies `#alias` as Package, consults the node_modules shadow, records index.ts, and grants Exact to the wrong `real`. Base has neither proof nor target.

Whether or not imports maps belong in the current coverage slice, unsupported syntax must not acquire an unrelated package proof. This input supplies an unambiguous native source winner.

**Fix:** distinguish package-import specifiers from ordinary package names. In modern modes where imports resolution is enabled, decline the package rung for `#` unless its native imports-map precedence and fallback have been proven. Preserve existing earlier paths authority. Add the native-winner/shadow negative witness. Full captured imports-map support is an alternative with more scope; it can be a later increment, with the S2 unsupported-status gate. A blanket decline is smaller but must not be described as proving native absence.

**Self-critique:** the physical `node_modules/#alias` shadow is an unusual adversarial layout rather than a normal npm install. It is nevertheless captured, constructible, and sufficient for a reachable false binding. This finding is scoped to modern enabled imports semantics; it does not assert the same precedence for Node10 or disabled imports resolution. The explicit native target and empty same-environment base rule out ownership and a pre-existing edge as explanations.

### W6 — Known null/no-result exports are treated as opaque winners

**WRONG · MATERIAL · confidence 100/100 for the demonstrated misses.**

**Evidence, reproduced:** `src/js_packages.rs:248-265`, `:274-275`, `:455-460`, `:476-481`, `:503-505`. These fixtures all have a TS-resolved indexed callable but no head module proof or target:

| Fixture | Input/state | TS result |
|---|---|---|
| `null-condition` / `null-condition-node16` | `exports:{"types":null,"default":"./index.ts"}` | `packages/lib/index.ts` through default |
| `exports-null-root` | `exports:null, main:"index.ts"` | Legacy `packages/lib/index.ts` |
| `null-subpath-outer-valid` | Closer `@ws/lib` maps `./feature` to null; outer link exports `./feature.ts` | Outer `packages/lib/feature.ts` |
| `null-root-outer-valid` | Closer package has exports:null and no implementation; outer link exports index.ts | Outer `packages/lib/index.ts` |

Null produces no search result in TS; a later condition or outer package may resolve. At the top level, null is not an active exports map. The prototype collapses null into `Json::Other`/`Found::Blocked`, treats presence as exports enablement, and returns from ancestor search without distinguishing known absence. `null-subpath-outer`, where neither package provides the subpath, correctly remains unresolved: the fix must not retry a lower-priority pattern inside the same map after selecting its literal null entry.

**Fix:** represent null distinctly from opaque/unsupported values; implement native no-result continuation, top-level legacy fallback, and ancestor/@types priority continuation where absence is proven. Keep declarations and opaque candidates as barriers. Add the listed positive and negative controls. Alternatively retain these as explicit unsupported classes and forbid S2 adoption for affected imports; OQ-3 already defers top-level null, but nested null and outer-package fallthrough need equal visibility. That alternative closes an adoption hazard, not PKG's completeness gap.

**Self-critique:** this is a false-negative finding, not a demonstrated false Exact in the current parked S2 artifact. Some null behavior is deliberately deferred by the bounded prototype. It still matters to the stated goal and cannot be used as proof that TS also fails. A sole null subpath with no eligible outer winner should stay unresolved.

### W7 — IMPLEMENTOR starts from the old uncommitted artifact, not the committed prototype

**WRONG · MATERIAL to executable dispatch · confidence 100/100 for a clean committed checkout. Evidence: static.**

`IMPLEMENTOR.md:3` says start on `plan/workspace-package-resolution` at base `4e592daa` with source/tests uncommitted. SPEC.md:5 and FILES.md:3 retain that state; HANDOFF.md:4, :11, :18 also describe the earlier dirty checkout. In the current supplied state, plan commit `c89bc5b7` and prototype commit `92c1d0bc` are separate children of main. A clean clone of the instructed plan branch contains **no `src/js_packages.rs`**, no package tests, and no `Resolver::package_in`. Thus dispatching the stated serial repair on that committed branch does not provide the artifact it says to preserve.

There is also a broken verification entry point: packet VERIFICATION.md:3 links to root `VERIFICATION.md`, which is absent from this plan checkout; the retained copy is `VERIFICATION-root.md` in the packet.

**Fix:** add a current committed-state dispatch binding: source/test base `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318`, plan packet `c89bc5b74df05e1f8746792c0cfad2805336b0f9`, and an exact controller-prepared checkout/composition instruction. Require source/test manifest parity before transferring receipts. Preserve the older handoff as dated history but reconcile current resume instructions and verification links. Alternative: publish a controller-composed source-plus-plan commit and pin it; this simplifies dispatch but is a controller Git action, outside this review.

**Self-critique:** the original dirty planning checkout may still contain all those files. Its historical report can be true as a dated observation. It is nevertheless insufficient for unambiguous dispatch from the committed prototype requested by this brief. No claim is made that an implementor has already run the wrong dispatch.

## SMELL findings

### S1 — Unsupported and native-unresolved outcomes need a distinct S2 adoption gate

**SMELL · MATERIAL. Evidence: static consumer risk plus reproduced PKG misses.** `SPEC.md:13`, OQ.md:5-6, IMPLEMENTOR.md:7, `src/js_packages.rs:330-357`, and `src/js_paths.rs:348-355` return the same `None` for bounded omissions and actual absence. The brief's S2-O9 excludes an import only when neither Prism **nor TS** resolves it. PKG's `None` does not establish the TS half. For example, the retained `array` fixture natively binds `index.ts` while PKG returns None; replacing the read with `import * as ns from '@ws/lib'; ns.real = replacement;` yields a possible writer import whose identity cannot safely be discarded under O9.

**Fix:** specify and enforce a pre-S2 coverage gate: either resolve each admitted writer class, retain unsupported/refusal status so it cannot justify exclusion, or supply independently bound native absence/resolution certificates for the actual writer population. An explicit richer result separates source proof, external/declaration winner, proven native absence, and unsupported authority. Alternative: keep S2 parked and resume only on a corpus subset with certified coverage; lower implementation cost, narrower adoption. Do not infer absence from workspace discovery or a missing generated output.

**Self-critique:** S2 is already parked and no refusal cut is adopted. I did not demonstrate a new S2 false Exact in this session. This is an adoption requirement, not a demand to implement every deferred feature in the current prototype repair.

### S2 — Fresh graph tests do not demonstrate persisted package cache invalidation

**SMELL · MATERIAL. Evidence: static test gap.** `tests/integration/js_packages_test.rs:85-106`, :188-199 re-read fixtures through `graph`; `tests/integration/js_paths_common.rs:11-15` always constructs a fresh repository/graph. Epoch assertions at `src/cpg_cache.rs:773` and `src/navigation/call_edge_cache.rs:723` demonstrate the one-time format bump, not subsequent metadata/config/link edits through cache hits.

**Minimal fixture:** warm CPG/nav caches for a linked package choosing index.ts; edit only exports to other.ts, then query the same cache and compare with a cold build. Repeat with only writer tsconfig condition/mode edits and link retargeting. No stale output was reproduced here.

**Fix:** add persisted full-hit/invalidation regressions for these dependency classes, including a newly present earlier declaration/opaque candidate. Existing topology machinery appears to retain the right facts (assessment below), so no speculative cache rewrite is recommended. Alternative: a retained external cache-hit/cold comparison gate; less unit-test coupling, but harder to maintain and bind to each revision.

**Self-critique:** absence of these tests does not show a cache defect. Static wiring and existing baseline topology tests may already make the implementation correct. This concern is not a blocker without a bad cached result.

## Unresolved boundary and what must precede S2

The following distinguishes TS-resolved misses from safe native failure. **“Before S2” means support or an enforced unsupported/certificate gate**, not permission to expand this repair into an unbounded full-Node implementation. Reproduced rows refer to the retained cases; other rows are static boundaries explicitly declared by source/spec.

| Feature/boundary | Evidence and consequence | Required before S2 / safe deferral |
|---|---|---|
| JS secondary pass, ancestor priority, @types | SPEC:13; `file` blocks JS. `closer-js-outer-ts` reproduces an outer **TS** winner hidden by a closer JS-only package. Existing `js-secondary` controls independently record a real native JS winner and deliberate PKG refusal. | Material writer imports; complete priority pass or explicit coverage gate before S2. Even TS-only admission needs correct outer-priority continuation. |
| Export arrays | `target`:261-263; `array` reproduces native index.ts and no PKG proof. | Can select an indexed writer module; before S2. Serial bounded increment is appropriate. |
| typesVersions / versioned types conditions | `legacy`:195; `target`:245. `typesVersions` and `versioned-types` reproduce native other.ts with no proof. | Before S2 for writer imports; declarations-only winners need an external/declaration outcome, never source redirection. |
| Package.json imports (`#` specifiers) | W5 demonstrates a false package binding instead of the native imports-map winner. | Refuse the unrelated package rung now; support or explicitly gate native-resolved writer imports before S2. |
| Nulls / failed inner exports | W6. | Before S2 for affected writers; keep literal-subpath selection distinct from outer search. |
| Bundler and Node16/Next ESM index fallback | `legacy`:215-218. `bundler-index` and `node16-esm-index` both natively bind index.ts and PKG misses. The latter extends OQ-3's stated bundler gap. | Before S2 for ordinary package writers; do not equate import condition with Node ESM extension rules. |
| Legacy subpaths through package links | Ancestor checks at :487-496 probe opaque lexical descendants before canonical lookup. `legacy-node10-subpath` and `esm-js-subpath-control` reproduce native feature.ts and no proof. | Add to OQ-3/IMPLEMENTOR explicitly; before S2 for these writers. Prove lexical sibling precedence, then use the captured link's canonical descendants. |
| Directory-valued types/main entries | `legacy`:209; `file`:171-175 treats the directory as a blocker. `types-directory` reproduces native src/index.ts, no proof. | Add to the continuation inventory; before S2 for these writers, with directory metadata/index priority controls. |
| Modern paths and proved-missing paths fallback | `resolve_in` supports node10 only; matching `package_in` paths are blanket refusals. `modern-paths-win` natively chooses other.ts; `paths-missing-fallback` natively chooses the installed index.ts. Both miss. | Preserve existing Lane-P barriers during this repair, but gate/support these classes before S2. A matched path is higher priority, not an unconditional TS terminal refusal when its candidates are proven absent. |
| baseUrl, moduleSuffixes, rootDirs, custom type roots/types, noResolve, preserveSymlinks, output/root options | Explicit presence barriers at :340-356; ownership barriers also apply. Some benign settings, including false/empty values, still decline. | Can occur on valid writer imports; certify coverage before S2. Do not silently relax earlier authority. Output/source remapping remains separately prohibited. |
| .mts/.cts entries and writers, inferred/jsconfig/solution owners, package/array extends, default/unsupported emit modes, omitted moduleResolution | SPEC/OQ and captured Config selector; extension entries block at :145. Existing default/amd controls witness native bindings with deliberate refusal. | Material where TS owns/resolves a writer; separate increments or a bounded certified population. |
| Numeric/duplicate package keys, unsupported target syntax, condition depth and snapshot budgets | Source deliberately declines; these can still be TS-readable/resolved cases. | Unsupported status is safe; treating it as native absence is not. Safely defer from the bounded prototype only with the S2 gate. |
| Declaration/generated/external winners | Correctly supply no implementation redirect. TS may still resolve a declaration or external module. | Preserve no-source-binding behavior; communicate resolved external/unsupported identity where S2 needs a writer join. No guessed source replacement. |
| npm/Yarn/pnpm/lerna declarations without installation | SPEC:15-17; inventory is separate from native lookup. Census walks named manifests, not a production workspace-declaration resolver. pnpm/lerna declaration expansion is expressly uncertified. | Discovery parsing can safely remain deferred for native binding: an uninstalled name gains no authority. Captured actual links qualify independently, including a pnpm-produced link into indexed source. |
| Generated-output redirects and loader schemes | OQ-1/2. Public native failures are distinguished from inventory; no loader certificate selected. | Native-unresolved imports are out of model under O9. Safely defer model expansion; requires owner authority. Builtin `node:` remains explicitly external and does not produce repository Exact targets. |

No blanket claim that all OQ-3 cases are harmless is admissible. Conversely, no evidence here supports inventing a binding merely because a workspace manifest names a package.

## Answers on discovery, cache wiring, measurements, and tests

**Discovery and competing packages:** the actual native lookup rung uses physical ancestor node_modules entries and modern self-reference, not a global same-name inventory. That is the right design. npm/Yarn workspace metadata does not bind uninstalled names; declared pnpm/lerna inventory expansion is not implemented. Captured package links may canonicalize into indexed source; outside/broken/opaque links decline. Competing same-name packages need nearest native priority, including failed-inner continuation (W6/table), rather than an “ambiguous inventory” tie or directory-name guess. Modern self-reference controls are native-witnessed; Node10 has no such rung. Package.json's `module` field is correctly ignored by TS; CompilerOptions.module is a different input.

**Cache assessment, static:** CPG/nav epochs 107/63 invalidate older serialized facts. Existing `JsPathsSnapshot::topology` (`src/js_paths_snapshot.rs:222-285`) hashes config bytes, package hashes, occupancy/probes, ambient/reference facts, external ancestry, and link inventory. `js_paths_boundary.rs:523-557` retains package bytes and hashes, including scanned installed metadata. `repo_loader.rs:224-292` primes writer import and caller-project export-hop dependencies before topology capture. This supports invalidation for package.json/tsconfig bytes, source/declaration occupancy, and link edits. I found no constructible invalidation failure; persisted behavior was not rerun (S2). No new live package read is introduced by js_packages; inherited occupancy absence probes can perform live metadata checks and should not be described as an entirely I/O-free resolver.

**Measurement admissibility:** rehashed the four retained public base/head full streams and confirmed byte equality directly: X and installed-X each contain 19,219 call sites, R 953, T 61,712 per retained summaries. Their denominators are complete per supplied loader/streams, not a selected changed-row sample. Census accounts separately for writer/spec pairs and occurrences, with paths-proof precedence; workspace inventory is not resolution causality. X and installed-X are two snapshots, not independent additive corpora. Installed-X's added module proof does not imply an Exact callable; the wrapper terminal remains unsupported. I checked all 42 retained changed control certificates for native module/owner and complete terminal span agreement; the summary totals are 58 cases, 42 correct changes, 16 unchanged/refused, zero lost/unproven. This is bounded evidence, and W1-W5 show uncovered semantics. The missing-binding population is not certified by “all changed rows correct.”

The full-suite receipt's log hash matches `final-test-binding.json`; parsing its 31 result records gives **5,147 passed / 0 failed / 1 ignored**. This is receipt verification, not a fresh suite run. Final input hashes match the reviewed prototype. The older nextest run remains pre-correction evidence. Advisory mutation selection remains 2/8, not an eight-witness acceptance claim. Public preservation is admissible for the retained snapshots; it is not a proof of universal preservation. Private-F aggregates in the review brief are supplied evidence only; I did not open private roots or transfer an independent F acceptance claim. The packet's historical “F unverified” statements should receive a controller addendum if those supplied aggregate receipts have now completed.

**Would each test fail on main?** The retained same-environment `base-red.log` contains seven executed behavioral failures (0/7), matching the seven existing-path tests. They each contain a positive package assertion that main misses. The eighth public classifier API does not exist on main; compile failure is not behavioral RED. Classification's unresolved scheme assertions and several standalone negative assertions would already pass on main. A composite test failing at its first positive assertion does not prove each later branch has a separate pre-change witness. The native controls cover self-reference/ESM/custom branches, while the review adds explicit negative/edge witnesses for W1-W5. Keep that distinction in the regression audit instead of inflating “seven RED” into coverage of every branch.

## Hypothesis/probe/result log

Each retained fixture has an expectation written before invocation. Wrong owner or usage mode was an alternative explanation; ProjectService consistently chose `tsconfig.json`, and the oracle reported the expected CJS/ESM mode. Base/head ran on the exact same fixture bytes and environment.

| Hypothesis / falsifier | Observation | Conclusion |
|---|---|---|
| Explicit .ts exports must not select absent-target .tsx; a native .tsx result falsifies. | Both native predictions matched; head produced wrong/unsupported .tsx Exact, base empty. .js control agreed. | W1 reproduced; ordinary .js substitution is not the cause. |
| Literal export stars remain literal; native index.ts would falsify. | Native unresolved, head index.ts Exact, base empty. | W2 reproduced; condition order cannot explain a string-only target. |
| Node16 ESM extensionless main is rejected; native feature.ts falsifies. | Native index.ts/null, head feature.ts Exact. Node10/CJS controls agreed. | W3 reproduced; loader mode distinguishes the inputs. |
| ESM extensionless barrel has no native callable; a callable declaration falsifies. | Node16/Next declarations empty, head impl.ts Exact; bundler callable/head agree. | W4 reproduced; entry binding itself is correct. |
| Modern #alias should select its imports-map target; native index.ts falsifies. | Native other.ts, head index.ts Exact, base empty. | W5 reproduced; unsupported specifiers must not enter unrelated lookup. |
| Null/no-result continuation can reach default/outer/legacy winners; native unresolved falsifies the positive cases. | Five misses matched predicted native winners; negative null-subpath control stayed unresolved. | W6 reproduced; null is not an opaque declaration winner. |
| Extensionless legacy ESM subpath might create a false binding. | Native and head both unresolved; explicit .js subpath resolves only natively. | No false-positive finding from this probe: lexical-link opacity prevents the initially suspected positive path. The completeness gap is retained in the boundary table. |
| Claimed final receipts bind to prototype bytes; source/hash differences falsify. | 547/447 input matches, four binary matches, matching full-suite log, equal retained public streams. | Receipts admissible within their stated source/population limits. |

## Not checked

No fresh Rust build/full suite, Tier-A matrix/quick/full corpora, MCP optional/all-features suite, mutation execution, broad public-corpus rerun, persisted cache-hit test, actual S2 writer-refusal execution, performance/RSS measurement, private F, off-machine custody, or exhaustive TS conformance. Formal npm/Yarn/pnpm/lerna discovery parsers were not executed; no claimed production parser exists for declaration-driven binding. All original public corpus/native input hashes were not revalidated against live corpus trees; public receipt checks above establish retained-artifact consistency. The 30 new witnesses do execute the pinned oracle and both retained artifacts live. No runtime mutation model, generated-output source authority, or virtual-loader policy was added.

## Verdict

**FIX — round 1 of 2.** Seven WRONG findings (all MATERIAL, including the dispatch defect), followed by two MATERIAL SMELL findings. W1-W5 violate the zero-false-binding/Exact goal with same-environment base controls. Repair their closed mechanisms, resolve or explicitly gate W6, and bind IMPLEMENTOR to the committed prototype. Keep the enumerated unsupported coverage gated before S2; broad PKG completeness and S2 adoption remain unapproved. No additional review round was dispatched and no cap was extended.
