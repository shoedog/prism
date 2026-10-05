# Lane PKG spec review — round 2 of 2, FINAL under the cap

**Recommendation: FIX. Six WRONG · MATERIAL findings; one SMELL · MATERIAL finding.** The original round-1 witnesses are repaired. The published 5,096-case differential also passes when rerun against the exact final artifact. Additional discriminating inputs nevertheless produce wrong module/Exact bindings and false `ProvenUnresolved` results. The three-way API is the right contract, but its absence arm is not yet safe for S2-O9.

These findings have concrete inputs and bounded fixes, including conservative `Unsupported` guards. Preserve the repaired artifact. This final review neither restarts it nor dispatches another round, and grants no S2 adoption.

## Binding and measured scope

- Clean working checkout: `/Users/wesleyjinks/code/prism-s2-review-sol`, branch `review-pkg`, HEAD **`fa3bcb027a1fd1b8d411ffe2eaa4f97ce616304d`**.
- Main: **`4e592daa7858a195eb3a9eb77c83dfbc763b49fa`**. Repaired prototype: **`03fa9c29f7b57fbc6c531d3287c37bdf95f4a9ff`**, following `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318`.
- Production/test references below mean **03fa9c29**, read through Git objects. The plan checkout does not contain the package implementation. Exact source copies are retained in [evidence/source](spec-r2-sol-evidence/source/).
- Rehashed all **547 production inputs and 447 test inputs** against committed prototype bytes: zero mismatches. All five manifest binary roles, all 16 probe files and the pinned TS oracle match. Evidence: [binding.json](spec-r2-sol-evidence/binding.json). This binds the retained compiled helpers to the supplied source manifest; it is not an independent rebuild.
- Oracle: TS **5.9.3**, `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`, SHA256 **`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`**. The packet oracle uses actual ProjectService ownership, effective compiler options and `getModeForUsageLocation`.
- Read both briefs, the original reviews, current SPEC §0/result contract, IMPLEMENTOR, R1-REPORT, MEASUREMENTS, CENSUS, PROBES, OQ, VERIFICATION, BUILD-MANIFEST, HANDOFF/FILES, the changed source/tests, differential/cache/public-audit scripts and the relevant TS implementation.

| Fresh execution | Cases/checks | Result |
|---|---:|---|
| Exact original Sol round-1 fixtures | 30 | 17 Bound / 5 ProvenUnresolved / 8 Unsupported; zero wrong; all original native predictions match |
| Final official differential | 5,096 | 2,242 Bound / 861 ProvenUnresolved / 1,993 Unsupported; zero wrong; 1,651 Unsupported cases bind natively |
| Additional boundary/authority/precedence cases | 104 | 62 affected cases: 49 false absences, 13 wrong module bindings and 13 false Exact edges |
| Same-environment main/prototype controls | 104 | Complete facts/CLI outputs retained for the constructed witnesses; attribution detailed below |
| Persisted CPG/nav cache gate | 12 | True warm hits, edit invalidation and complete warm/cold byte equality all pass |

The 104 cases are three separate populations, not extra rows silently folded into the published denominator. A module and its callable edge are two rejected observations from one case. See [case-summary.json](spec-r2-sol-evidence/case-summary.json), [hypothesis/probe/result log](spec-r2-sol-evidence/hypothesis-probe-results.md) and the complete `wrong.json` files. Every rejected population was read before follow-up probes. No source repair was attempted.

## Round-1 disposition

The 30 **original on-disk inputs**, rather than only regenerated approximations, were rerun with the final helper and oracle: [r1-original-rerun](spec-r2-sol-evidence/r1-original-rerun/). Sol W1–W5 no longer create their false edges; W6's four native-resolved null/outer-continuation cases now bind correctly. Supported positive and native-absence controls remain correct. Arrays, typesVersions, versioned types, modern ordinary paths, failed-path fallback, directory fields, JS-secondary priority and `#imports` remain explicit `Unsupported`, including all eight original native-bound omissions. This is a legitimate bounded disposition, not proof of native absence.

The official final matrix independently reruns the defined Opus cases, including missing legacy declarations, writer-extension mode, ESM dependency fields, outer package scope, self-name with exports disabled and colon paths. The repaired source mechanisms agree with those results. Ten Opus IDs without definitions remain unverified, as the packet explicitly records; I did not invent their fixtures. Sol W7's dispatch and broken verification link are fixed, as discussed below. No inherited WRONG was downgraded to a SMELL.

## WRONG findings

### W1 — Modern colon paths reuse implicit Node10 extension/index probing

**WRONG · MATERIAL · confidence 100/100. Evidence: reproduced.** `src/js_paths.rs:395–399` newly permits colon paths in Node16/Next, but `:451–452` still calls the ordinary `prove_path`. It does not apply the writer's Node ESM file policy before returning `Bound` at `:350–351`.

Minimal fixture (`extra-fixtures/colon-node16-module-shim`):

```json
// package.json
{"type":"module"}
// tsconfig.json
{"compilerOptions":{"moduleResolution":"node16","module":"node16",
 "paths":{"virtual:local":["./shim"]}},"include":["**/*"]}
```

```ts
// app.ts
import {real as picked} from 'virtual:local';
export function run(){picked();}
// shim.ts
export function real(){return 1;}
```

TS reports owner `tsconfig.json`, mode **99/ESNext**, and **no module or callable**. R1 binds `shim.ts` and grants Exact `real`. Mapping to `./dir` with `dir/index.ts` produces the same defect. Both variants reproduce in NodeNext: four cases, each with a wrong module and Exact edge. Bundler, CommonJS writer and explicit `./shim.ts` controls agree with TS.

**Attribution/control:** fresh main and 92c1d0bc CLI/facts on these exact fixtures contain no proof or target; the R1 CLI reproduces the helper's false Exact. This is an **R1 regression**, not a helper-only result. See [extra-results/wrong.json](spec-r2-sol-evidence/extra-results/wrong.json), [controls.json](spec-r2-sol-evidence/controls.json) and `controls/colon-*/`.

**Fix:** apply actual writer/barrel usage mode to colon-path candidate loading. In Node ESM, do not append extensions or guess directories. A small safe alternative is to decline these candidates; the existing matched-path barrier then returns `Unsupported("paths authority")`. Supporting native explicit JS substitution is a broader alternative, with more yield but more probing rules. Tests must retain both ESM negatives, CommonJS/bundler positives and explicit-extension controls, including an export hop using the caller project's options.

**Self-critique:** this does not affect the reproduced explicit-TS C28 cases or ordinary modern non-colon paths, which remain fenced. It requires a Node ESM usage and an implicit target. The oracle's owner/mode and passing CommonJS/bundler controls rule out wrong-project selection and a universal paths rejection as explanations. Confidence collapses if this exact native ESM input is shown to resolve; the retained native result and real CLI presently agree on the failure.

**Cap classification:** new reachable instance of the already known modern-mode probing family; closed, bounded fix.

### W2 — Trailing-slash exports keys are ignored, allowing a lower wildcard to win

**WRONG · MATERIAL · confidence 100/100. Evidence: reproduced.** `src/js_packages.rs:396–414` considers only keys with `*`; `:416–418` selects among that incomplete set. TS 5.9.3 `loadModuleFromExportsOrImports` (`typescript.js:45984–46027`) also considers trailing-slash keys and ranks them together with patterns.

Bundler fixture `folder-fixtures/folder-bundler-positive` imports `@ws/lib/feature/item.js` from this installed workspace package:

```json
{"name":"@ws/lib","exports":{
 "./feature/":"./src/",
 "./*":"./other/*.ts"
}}
```

Both `src/item.ts` and the literal lower-pattern candidate `other/feature/item.js.ts` exist and export `real`. TS selects **`packages/lib/src/item.ts`**; PKG binds **`packages/lib/other/feature/item.js.ts`** and grants Exact to its different declaration. Removing `src/item.ts`, or replacing the folder target with null, makes TS unresolved while PKG still selects the lower wildcard. All three variants reproduce in Bundler, Node16 and NodeNext: nine false module/Exact cases. Folder-only maps additionally produce false `ProvenUnresolved` while TS binds; six such cases occur in the boundary/extra populations.

**Attribution/control:** main has no target; the same-environment 92c1d0bc prototype already has the wrong lower-pattern Exact, and R1 retains it. This is a newly demonstrated **inherited PKG defect**, not an R1 edge regression. See [folder-results/wrong.json](spec-r2-sol-evidence/folder-results/wrong.json) and [folder-controls.json](spec-r2-sol-evidence/folder-controls.json).

**Fix:** include supported folder keys in native ranking, carry their remainder separately from wildcard capture, and append it to a slash-terminated target. A selected missing/null folder must not retry a lower key. The smaller alternative is `Unsupported` for maps with potentially applicable folder keys, **before selecting a wildcard**. Guarding only an unmatched request fixes the false absence but leaves the false bindings. Full support gives source yield; refusal keeps this increment smaller. Retain positive, missing, null, overlapping-pattern and ordinary-pattern controls.

**Self-critique:** trailing-slash maps are uncommon legacy metadata, but they are native TS 5.9.3 behavior, and no package-syntax fence currently covers them. The witness supplies the exact expanded wildcard filename, so it is not a vacuous absence comparison. This does not dispute ordinary single-star ordering. Demonstrating that native TS chooses the lower wildcard on these exact fixtures would refute the finding; all nine retained results select the folder or no target.

**Cap classification:** new review family (folder-map authority/precedence), with a closed guard or implementation fix.

### W3 — An absent exact TS export bypasses the JS-secondary uncertainty check

**WRONG · MATERIAL · confidence 100/100. Evidence: reproduced.** `src/js_packages.rs:197–199` returns directly from `present` for exact TS/declaration exports. An absent literal therefore bypasses the JS-secondary checks at `:239–251`. The final ancestor result at `:738–739` can falsely certify absence.

Fixture `boundary-fixtures/export-bundler-ts-js-only`: Bundler/esnext, linked `@ws/lib`, `exports:"./index.ts"`, **no index.ts**, and **existing index.js** exporting `real`. TS resolves canonical `packages/lib/index.js`; PKG reports **`ProvenUnresolved`**, owner `tsconfig.json`. Analogous `.tsx`, `.mts`, `.cts`, `.d.ts`, `.d.mts` and `.d.cts` targets with their native JS-family fallback reproduce in Bundler, Node16 and NodeNext: **21 false-absence cases**.

The original W1 fix correctly makes the **preferred TS pass** probe a literal TS export. It does not prove the subsequent native JS pass absent. In TS, `loadFileNameFromPackageJsonField`'s exact branch depends on the active extension mask (`typescript.js:45450–45460`); the secondary pass can use `tryAddingExtensions`' JS branch. See [boundary-results/wrong.json](spec-r2-sol-evidence/boundary-results/wrong.json).

**Fix:** preserve native uncertainty across the missing preferred pass and check the eligible secondary counterparts before declaring absence. Keep the original literal-TS/no-sibling-TS repair intact. Do not bind JS merely because it exists; full ancestor/condition priority is still unimplemented. A safe bounded alternative is `Unsupported` when this secondary pass has not been discharged. Broadly fencing missing explicit TS exports costs some valid absence certificates but requires no JS-source admission. Add JS-only witnesses, neither-file absence controls, exact-existing TS positives and a later preferred TS condition to preserve priority.

**Self-critique:** a JS winner is deliberately outside current package-source binding support, so this is a classification defect, not a demand to bind JavaScript. No false current callable is asserted here. Both prechange binaries have no source proof on these cases; they have no three-way API, so their `None` is not evidence of prechange native-absence certification. ProjectService owner/mode are retained. A native-null result on the JS-only fixture would refute this finding; the oracle binds each of the 21 counterparts.

**Cap classification:** closed instance of extension/secondary-pass authority; the early-return mechanism is in the R1 repair.

### W4 — Legacy preferred types fields hide a distinct secondary-pass main winner

**WRONG · MATERIAL · confidence 100/100. Evidence: reproduced.** `src/js_packages.rs:275–277` chooses a single `typings`/`types`/`main` field. This models the preferred declaration/TS pass, but reusing it as the complete lookup never inspects a distinct `main` field when TS reaches its secondary pass.

Fixture `boundary-fixtures/legacy-types-js-other-main-node10`: linked library metadata is:

```json
{"name":"@ws/lib","types":"types.ts","main":"main.js"}
```

Only `main.js` exists and exports `real`; `types.ts`, its siblings and root index candidates are absent. The writer imports `real` from `@ws/lib`. TS binds **`packages/lib/main.js`**; PKG says **`ProvenUnresolved`**. The same result reproduces in Bundler, Node16 and NodeNext: four cases. By contrast, the four `legacy-types-js-only-*` controls correctly return `Unsupported` when `index.js` is a same-stem candidate; that guard does not cover distinct main fields.

TS selects its field using the active extension mask in `loadNodeModuleFromDirectoryWorker` (`typescript.js:45745–45755`): declaration-inclusive lookup prefers types, while JavaScript lookup uses main. Evidence is in [boundary-results](spec-r2-sol-evidence/boundary-results/); all four same-environment main/prototype facts remain unbound.

**Fix:** after no preferred result, retain the distinct native main/secondary field authority before emitting `ProvenUnresolved`. A narrow `Unsupported` barrier when an unproved main candidate can win is sufficient; JS admission can remain parked. An even smaller alternative fences legacy preferred-field misses with a distinct main field, losing some safe absence certificates. Full two-pass metadata selection provides more yield but is a separate increment. Tests should cover different-stem main, main absent, preferred TS present, preferred index TS winning ahead of JS, and the existing same-stem guard.

**Self-critique:** this requires a miss of all preferred candidates and a distinct existing native main winner. It does not apply when types resolves or when an eligible preferred index source wins. There is no prechange status API, so this is a current contract violation rather than an attributed prechange classification regression. Inspecting only the selected types stem cannot establish native absence; the oracle's distinct main result is the discriminating observation.

**Cap classification:** newly enumerated metadata-selection instance of JS-secondary authority, with a bounded refusal fix.

### W5 — JSON winners are omitted even when native JSON resolution is enabled

**WRONG · MATERIAL · confidence 100/100. Evidence: reproduced.** `src/js_packages.rs:212` probes a JSON declaration replacement; `:240–246` supplies no actual JSON candidate. `resolveJsonModule` is not fenced in the option guards (`:529–545`), and `Search` carries no effective JSON-resolution authority. A missing declaration replacement can thus become `ProvenUnresolved` despite an existing native JSON module.

Fixture `boundary-fixtures/json-bundler`: linked `@ws/lib` with `exports:"./index.json"`, `main:"index.json"`, existing `index.json`, and `resolveJsonModule:true`. TS resolves **`packages/lib/index.json`**; PKG says **`ProvenUnresolved`**. Explicitly enabled JSON roots and explicit `@ws/lib/data.json` legacy subpaths reproduce in all four supported modes. The omitted-option controls also reproduce in **Bundler and NodeNext**, where TS 5.9.3 enables JSON by default (`typescript.js:22026–22042`). There are ten demonstrated false absences. Node10/Node16 default-disabled controls remain natively unresolved.

**Fix:** compute effective JSON eligibility, including defaults, and keep an existing/opaque eligible JSON candidate `Unsupported` after preferred declaration probing. JSON is unindexed module authority, not a callable implementation. An alternative is a conservative JSON-target refusal independent of the option, costing some provable native absences. Merely fencing explicitly written `resolveJsonModule:true` misses the Bundler/NodeNext default cases. Supporting JSON module identity would need a separate non-callable representation. Retain explicit true/false/default controls and declaration-shadow cases.

**Self-critique:** this is not a request for an Exact edge to a JSON object. The imported-name callable diagnostics do not affect the independently observed native module binding. Current package binding support may omit JSON, but the result must then be `Unsupported`. Main/prototype have no source proof; they cannot serve as a prechange three-way-status control. Native-null results with the actual effective JSON option would refute the finding; the retained native targets demonstrate the opposite.

**Cap classification:** new review family (JSON authority/default options), closed candidate/refusal fix.

### W6 — The @types fence checks the directory but misses declaration sibling files

**WRONG · MATERIAL · confidence 100/100. Evidence: reproduced.** `src/js_packages.rs:721–730` checks only occupancy of `node_modules/@types/<mangled-name>`. TS's declaration lookup can resolve its sibling **`<mangled-name>.d.ts`** without that directory existing (`typescript.js:46340–46415`, `loadModuleFromSpecificNodeModulesDirectory` → `loadModuleFromFile`).

Fixture `extra-fixtures/types-sibling-node10` has no workspace implementation or package directory, but contains:

```ts
// node_modules/@types/ws__lib.d.ts
export declare function real():void;
// app.ts
import {real as picked} from '@ws/lib';
export function run(){picked();}
```

TS binds **`node_modules/@types/ws__lib.d.ts`**; PKG reports **`ProvenUnresolved`**. Unscoped `lib` → `@types/lib.d.ts` and both names in Bundler, Node16 and NodeNext reproduce: eight cases. The corresponding actual `@types/ws__lib/index.d.ts` directory controls correctly produce `Unsupported("@types authority")` in every mode.

**Fix:** discharge the native declaration-file candidates as well as package-directory occupancy before certifying @types absence, with the actual scoped-name/subpath rules and mode. Any possible declaration winner must remain `Unsupported`; it is not an implementation redirect. A broader conservative @types refusal where sibling authority is not proven is an alternative, with less absence yield. Retain scoped/unscoped sibling positives, directory controls, truly absent candidates and earlier preferred source winners.

**Self-critique:** the sibling-file layout is less common than an installed @types package directory, but native TS explicitly supports it. The cases use no `types`/`typeRoots` override and actual configured writers, so option fencing does not explain them away. This does not show that the working directory-case fence is broken. There is no old status API; this is a demonstrated current false certificate. The directory/sibling native and PKG contrast isolates the missing file probe.

**Cap classification:** closed instance of the already declared @types-unsupported boundary; no new @types binding implementation is required.

## SMELL finding

### S1 — The generated product lacks discriminating secondary/authority inputs

**SMELL · MATERIAL. Evidence: static coverage audit plus reproduced counterexamples.** `probes/differential-generate.py:43–63` crosses 15 shapes, but most implementation candidates are `.ts`; the product's writer-extension axis does not vary the entry's native secondary winners. There is no JSON winner, @types sibling, trailing-slash exports key, or distinct missing-types/existing-main-JS shape. Writer/dependency package types are coupled. Colon witnesses (`:94–96`, and `reviewer-cases.json`) use explicit TS targets and never contrast extensionless/index paths under Node ESM.

The checker is functional: every added wrong observation is rejected by the **unchanged committed runner**. The official run is not wholly vacuous: its Bound, native-absence and native-bound Unsupported populations are nonempty, and all 30 original expected targets agree. The missing discriminating files/metadata nevertheless leave the six mechanisms outside the acceptance population. For example, the official missing `./dist/index.ts` has no JS counterpart in dist; it cannot expose W3.

**Fix:** add the retained six-family witnesses and their negative controls to the committed generator, and cross target extension/presence/secondary winner independently of writer extension. Separately vary writer/dependency type for modern probing. This need not implement omitted native features: expected `Unsupported` is sufficient to test their status boundary. A bounded alternative is an explicit allowlist for proven-absence classes, refusing all unproved package misses; it trades absence yield for a smaller soundness obligation. A full conformance harness is a larger alternative, not required by this review.

**Self-critique:** the published totals are true for their stated bounded population; this is not an accusation of fabricated measurements or a separate blocker in addition to W1–W6. No finite matrix proves complete TS conformance. The useful repair is discriminating coverage for newly guarded paths and disciplined uncertainty, not simply increasing the case count.

## Cache, lane-P, public evidence and dispatch

The committed cache probe was read and freshly executed against the hash-bound R1 CLI: [cache-rerun/summary.json](spec-r2-sol-evidence/cache-rerun/summary.json). All 12 checks pass, including real CPG and nav-sidecar warm hits, metadata/condition/config edits, link retargeting, declaration creation/removal and outer-scope type/malformed/restored changes. Both persisted artifacts invalidate and complete warm/cold output matches. No stale-cache defect was reproduced. This closes my original cache-test SMELL for the tested population.

The relative export-hop change retains caller-project options and applies per-barrel ESM mode; the original modern-barrel negatives and Bundler positive now agree with TS. Ordinary Lane-P ownership and failed-path barriers remain. W1 identifies the newly broadened colon-path interaction that still bypasses that mode policy. The inherited ordinary Lane-P JSX issue is explicitly parked in OQ; it is not silently counted as repaired here.

I reread and compared the **complete retained** public main/head call streams: X 19,219; installed-X 19,219; R 953; T 61,712 sites. Every stream is byte-identical. The public module-audit script was freshly executed: retained source sets/hashes agree, no main module proof is lost or changed, and the sole installed-X addition receives a fresh native target/project certificate. Evidence: [public-stream-audit.json](spec-r2-sol-evidence/public-stream-audit.json), [public-module-audit.json](spec-r2-sol-evidence/public-module-audit.json). These are an audit of supplied public executions plus a fresh certificate, **not fresh CLI execution of the four corpora**. X and installed-X are two snapshots, not independent populations.

**Committed dispatch is now unambiguous.** `IMPLEMENTOR.md:3–5` pins distinct source and packet bases, explicitly applies their separate R1 patches, and requires manifest parity. The final source/test/probe hashes match 03fa9c29/fa3bcb02. It no longer asks a clean plan-only branch to supply package code. `VERIFICATION.md:3` points to the existing packet-local `VERIFICATION-root.md`. A simpler controller handoff could now pin 03fa9c29 and fa3bcb02 directly, but that is optional; the exact old-base-plus-patch construction is well specified. WRONG repairs precede new JS-secondary work, S2 remains parked, and workers have no Git/F/adoption authority.

## Final cap classification and bounded repair order

This is **round 2 of 2**, not the start of another loop. The original reported WRONG fixtures are repaired; this review exposes newly discriminated authority classes as well as closed instances of known mode/extension/@types families. Each W1–W6 has an input, incorrect result and a bounded guard or implementation fix. No finding requires a source restart or whole-feature rewrite. I do not claim that the wider resolver has reached exhaustive convergence.

1. Remove the false positive paths: W1's ESM colon guesses and W2's incomplete folder/wildcard selection.
2. Close the false absence arms: W3–W6. Keep native-resolved but unindexed/unsupported winners as retained refusals. If narrowly proving all absence branches is too broad for the continuation, use the conservative package-miss `Unsupported` alternative and keep the audited URI policy separate.
3. Add the discriminating controls to the differential generator; preserve all original repairs and public/lane-P base edges. Run required implementation checks on the resulting artifact, with explicit source-bound RED/GREEN receipts.

The controller must decide any continuation at the cap explicitly. This review performs no third-round dispatch, source mutation, S2 execution or adoption. The result is **closed enumerable FIX**, including newly reviewed families; it is not evidence that every future native semantic family has been enumerated.

## Not checked

- No independent source build, Rust full-suite execution, fmt/clippy/mutation gate, Tier-A quick/matrix/full corpus or S1b rerun in this review. The supplied final MCP **5,163 passed / 0 failed / 1 ignored** and other gate receipts remain inherited, bound to matching supplied source inputs, not my executions.
- No full new materialization of all generated fixtures; the final gate was rerun without `--reuse` using the final receipt's hash-bound `matrix-accepted/manifest.json`. A first 4,760-case historical `matrix-final` rerun was green but is separately identified in the log and is not used for the final 5,096 claim.
- No fresh four-corpus CLI execution, no private F access or independent F verification; the brief's F result remains supplied controller evidence.
- No S2 scratch rebuild, end-to-end S2-O9 failure execution, refusal adoption or merge. False absence is demonstrated in PKG's public API; its downstream effect follows the supplied S2-O9 contract.
- No exhaustive TS conformance, performance/RSS assessment, concurrent filesystem mutation proof, off-machine backup, or formal pnpm/lerna discovery certification.
- No invented coverage for undefined Opus C3, C15, C17–C21, C23, C24, C26; no expansion of parked ordinary Lane-P JSX or other bounded unsupported features.

All evidence and scripts are retained under `spec-r2-sol-evidence/`, with a living [review handoff](spec-r2-sol-handoff.md). The checkout remains clean; no Git writes, network, source edits or build occurred. No `target/` was created, so no build directory requires deletion.

**Verdict: FIX.**
