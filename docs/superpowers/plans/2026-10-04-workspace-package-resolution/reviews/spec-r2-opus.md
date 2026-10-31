# Lane PKG spec review, round 2 of 2 (Opus): workspace package-entry resolution after R1

**Reviewed:** plan `review-pkg` = `fa3bcb02` (R1-REPORT, SPEC §0–3, IMPLEMENTOR, OQ, probes/). Prototype `origin/proto/workspace-package-resolution` = `92c1d0bc` + `03fa9c29`, extracted with `git archive` into `target/proto` and built release, offline. I checked that `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/R1-src.patch` applied to `92c1d0bc` produces a tree byte-identical to `03fa9c29`.

**Goal (from the round-1 brief):** bind JS/TS bare specifiers to in-repo workspace package entries under TS 5.9.3 semantics, with zero false module bindings, zero false Exact edges and zero lost base edges. Under S2-O9, only `ProvenUnresolved` may be treated as out of model, so a `ProvenUnresolved` where TS binds is as serious as a false binding.

**Oracle:** pinned TS 5.9.3 `typescript.js`, SHA256 `3ae902c9…7675`. All new cases were run through the packet's own harness: `probes/differential-oracle.cjs` (real ProjectService owner, `getModeForUsageLocation`, `resolveModuleName` with that mode, and checker declarations), plus `probes/dump-resolution.rs` built as a temporary example against `target/proto`. Comparison used `probes/differential-run.py` unchanged. The production CLI (`prism nav --no-cache callees --symbol run`) was used as a second channel and for the same-environment base control.

## Priority 1: are the round-1 WRONGs fixed?

**Yes, all of them.** I re-ran the packet generator (`differential-generate.py`) and the gate in this environment. It reproduced R1's totals exactly: **5,096 cases, 2,242 Bound / 861 ProvenUnresolved / 1,993 Unsupported, 0 wrong, 1,651 Unsupported where TS binds**, and the same unsupported table. Results for each of my round-1 fixtures in that run:

| Round-1 finding | Fixture(s) | TS 5.9.3 | R1 prototype |
|---|---|---|---|
| F1 | C1, C11 | b.ts; unresolved | b.ts; ProvenUnresolved |
| F2 | C2 | src/index.ts | src/index.ts |
| F3 | C5, C6, C6b | other.ts; index.ts; index.ts | same |
| F4 | C4 | index.ts | index.ts |
| F5 | C22 | other.ts | other.ts |
| F6 | C25 | other.ts | other.ts |
| F7 | C28 (node10 and bundler, `node:url` and `virtual:pwa`) | shim/*.ts | same, Exact |
| F9 | C7, C8; C9, C10, C12, C13, C29, C30 | binds | Unsupported (package imports / paths authority); Bound, agreeing with TS |
| F12 | C16 | index.tsx (6142) | Unsupported (JSX) |

Other checks:
- **Package tests:** `cargo test --release --test integration js_packages_test::` gives 24 passed, 0 failed.
- **Persisted-cache spot check:** C22 run with a persistent `--cache-dir` (cold, then warm), then the outer `package.json` type flipped to commonjs and back. The callee followed other.ts → other.ts → index.ts → other.ts.

On R1-REPORT's note that the Opus review gives no definitions for C3, C15, C17–C21, C23, C24 or C26: those IDs were gaps in my fixture numbering and never findings. Nothing is missing from coverage.

---

## WRONG findings

### G1. A JSON winner is reported as `ProvenUnresolved` while TS binds the `.json` file (WRONG, MATERIAL: S2-critical direction)

**Evidence:** in `src/js_packages.rs` `Search::file` (`src/js_packages.rs:186`; `.json` arm at :212, secondary-pass vector at :240-251), a `.json` suffix maps to `(stem, [".d.json.ts"])`. The secondary-pass vector `js` has **no `.json` arm**, so a present `stem.json` is never seen and the function returns `Found::Absent`. That becomes `Resolution::ProvenUnresolved` at `result()`.

TS's `tryAddingExtensions` case `.json` (typescript.js:45478-45479) tries `.d.json.ts` and then, when `extensions & Json`, `.json`. `resolveJsonModule` is on by default for bundler and for `module: nodenext`, through computed options at typescript.js:22026-22038 (the oracle confirms both). TS also tries `.json` only after the TS/declaration pass fails. `r2-json-cond-fallthrough` agrees: `types: ./data.json, default: ./index.ts` resolves `index.ts` in both TS and prism. So this is a missing secondary pass, exactly parallel to R1's "JS secondary priority pass" guard.

**Reproduced** with the packet harness (writer `app.ts`, link `node_modules/@ws/lib -> ../../packages/lib`). All seven cases are flagged `false absence` by `differential-run.py`:

| Case | Options and metadata | TS 5.9.3 | Prototype |
|---|---|---|---|
| r2-json-bundler | bundler, `resolveJsonModule:true`, `exports "./data.json"` | packages/lib/data.json | ProvenUnresolved |
| r2-json-bundler-default | bundler (option unset), same exports | data.json | ProvenUnresolved |
| r2-json-nodenext-cjs | nodenext, CJS scope, same exports | data.json | ProvenUnresolved |
| r2-json-node10-main | node10, `resolveJsonModule:true`, `main "data.json"`, no index | data.json | ProvenUnresolved |
| r2b-json-legacy-subpath | node10, spec `@ws/lib/data.json` | data.json | ProvenUnresolved |
| r2b-json-pattern | bundler, `exports {"./*":"./*.json"}`, spec `@ws/lib/data` | data.json | ProvenUnresolved |
| r2b-json-pkg-subpath | bundler, no exports, spec `@ws/lib/package.json` | packages/lib/package.json | ProvenUnresolved |

**Base control:** prototype only. Main has no package rung, and the round-1 prototype returned `None`, which was not a native-absence claim. The defect is the `ProvenUnresolved` label, which SPEC §1/§3 says must never coexist with a native module ("Every ProvenUnresolved must have no native module").

**Fix (recommended):** in `file()`, add `.json` to the secondary-pass vector. When `stem.json` (or `p`, for an explicit `.json` target) is not `first_pass_absent`, return `Blocked("JSON secondary pass")`. Do this unconditionally, or only when the effective `resolveJsonModule` is true, using TS's computed default: explicit value, else bundler or `module` node20/nodenext. The unconditional form is simpler and only conservative.

**Alternative:** implement JSON as Unsupported("json winner") explicitly in `present()`. That costs the same and keeps the reason distinct in the gate table.

**Self-critique:** a JSON module exports no callables, so this cannot produce a false Exact edge to a *JSON* callable. The S2 harm is the contract breach: S2-O9 would treat a TS-resolved import as out of model, and a name-based rung could then grade a same-named call Exact. `import pkg from '@ws/lib/package.json'` is common, though calls through it are not. Treat this as a hard gate failure (the SPEC's own acceptance rule) with low practical yield.

### G2. Exports folder-mapping keys (`"./sub/"`) are ignored, giving a false absence and, with a competing `"./*"` key, a false Exact (WRONG, MATERIAL)

**Evidence:** in `Search::entry` (`src/js_packages.rs:370`, candidate filter at :399), pattern candidates are built only from keys where `split_once('*')` succeeds. TS `loadModuleFromExportsOrImports` (typescript.js:45993) collects keys with `hasOneAsterisk(k) || endsWith(k, "/")` and sorts them with `comparePatternKeys`. That sort ranks a no-star key by its full length, so `"./sub/"` (6) outranks `"./*"` (3). A trailing-slash key then matches by `startsWith` (typescript.js:46016-46025), and its target is `target + subpath` (46115).

**Reproduced:**

| Case | Mode | exports / spec / files | TS 5.9.3 | Prototype |
|---|---|---|---|---|
| r2-trailing-slash | bundler | `{"./sub/":"./src/"}` / `@ws/lib/sub/feature.js` / src/feature.ts | packages/lib/src/feature.ts | **ProvenUnresolved** (false absence) |
| r2-trailing-slash-nodenext | nodenext | same | src/feature.ts | **ProvenUnresolved** |
| r2-trailing-slash-vs-star | bundler | `{"./sub/":"./a/","./*":"./b/*"}` / `@ws/lib/sub/x.js` / a/x.ts, b/sub/x.ts | packages/lib/a/x.ts | **Bound b/sub/x.ts, Exact** (wrong module and callable) |

**Base control:** the production CLI shows `b/sub/x.ts` at score 1.0 on both `92c1d0bc` and `03fa9c29`. This is inherited from the prototype, not introduced by R1, and main has no package rung. Round 1 checked pattern ordering and missed folder keys; this is my miss as well.

**Fix (recommended):** if any dotted exports key ends in `/`, return `Blocked("exports folder mapping")` before pattern selection. This is one predicate and only conservative. Do the same in self-name exports, which goes through the same `entry()`.

**Alternative:** implement the folder rule. Include `/`-suffixed keys in the candidate list, sort them with TS's `comparePatternKeys` (baseLen = star index + 1, or full length), and for a winning folder key require a string target ending in `/` (otherwise TS treats it as invalid: no result). Then probe `target + subpath` with exact semantics. This adds yield, but the mapping is deprecated, so the extra yield is small.

**Self-critique:** folder mappings are deprecated in Node and rare in TS-first workspace packages. Older packages still ship them, and the false-Exact variant needs one together with a `./*` pattern. It is low prevalence, but it is a demonstrated false Exact on the production path.

### G3. R1 regression: colon specifiers through `paths` in Node16/NodeNext ESM writers bind with node10 extension and index probing that TS disables (WRONG, MATERIAL)

**Evidence:** R1 changed `Resolver::resolve_in` (`src/js_paths.rs:396`) to admit colon specifiers in `node16`/`nodenext`/`bundler`. The proof then runs lane P's node10 `prove_path`, which adds `.ts`/`.tsx`/`.d.ts` and the directory `index`. TS applies `paths` through `nodeLoadModuleByRelativeName` → `loadModuleFromFile`. Under `EsmMode`, that adds **no** extension (typescript.js:45429-45434) and **no** directory lookup (45399-45401). R1 respected EsmMode for package fields and export hops (W4/`relative_esm`) but not for this newly admitted branch.

**Reproduced:**

| Case | Mode / scope / writer | paths → file | TS 5.9.3 | Prototype |
|---|---|---|---|---|
| r2-paths-esm-colon | nodenext, root `type:module`, app.ts | `virtual:*` → `./shim/*`, shim/pwa.ts | unresolved (2307) | **Bound shim/pwa.ts, Exact** |
| r2-paths-esm-colon-dir | same | `virtual:x` → `./shim/x`, shim/x/index.ts | unresolved | **Bound shim/x/index.ts, Exact** |
| r2-paths-mjs-colon | nodenext, CJS root, **app.mjs** | `virtual:*` → `./shim/*` | unresolved | **Bound shim/pwa.ts, Exact** |
| r2-paths-esm-node-colon | node16, `type:module` | `node:url` → `./shim/url` | unresolved | **Bound shim/url.ts, Exact** |

Mode-correct controls agree with TS:
- `r2b-paths-bundler-colon-dir` (bundler) binds index.ts in both;
- `r2b-paths-node16-cjs-colon` (node16, CJS scope) binds shim/pwa.ts in both;
- `r2-paths-esm-colon-js` (ESM, target `./shim/*.js`) is bound by TS and Unsupported in prism, which is conservative.

**Same-environment base control:** with the production CLI on `92c1d0bc` (built in this environment), all four cases show the callee as the import binding in the writer (app.ts / app.mjs), with no shim binding. On `03fa9c29` they show shim/pwa.ts, shim/x/index.ts and shim/url.ts at score 1.0. R1 introduced the regression.

**Why the gate missed it:** C28 is generated only for `node10` and `bundler` (`differential-generate.py`, final loop). Paths are never crossed with mode × writer extension × package type.

**Fix (recommended):** in the modern colon branch of `resolve_in`, compute `usage_mode` for the writer. When `node_esm`, either return `None`, so `package_in` returns `Unsupported("paths authority")`, or prove with `relative_esm`-style probing (explicit-extension substitution only, no implicit extension, no index). Apply the same rule to the non-relative branch of `hop()`, which reuses `resolve_in`. Add the four witnesses plus the two controls.

**Alternative:** admit colon paths only for bundler and node10 until lane P has a mode-aware proof. This is simplest and costs only the rare ESM colon-alias yield.

**Self-critique:** tsconfig colon aliases are rare, and pairing them with Node ESM is rarer still. This is a provable false Exact on the production path, introduced by the repair of my own F7. The fix is one mode check.

---

## SMELL findings

### S1. The differential gate cannot falsify most `ProvenUnresolved` claims, and its axes miss the classes above (SMELL, MATERIAL)

The gate is rerunnable and the oracle is invoked correctly. Two checks:
- I reproduced the totals byte-for-byte, and the run takes minutes.
- It uses the writer's real ProjectService owner options, the per-file usage mode and checker declarations.

But the population is far less discriminating than "5,096 cases" suggests, in four ways:

1. **Absence is untested against competitors.** All 15 shapes use only `.ts`/`.tsx`/`.d.ts` files, and every ProvenUnresolved in the product comes from those shapes: missing-target 195, literal-star 195, six legacy/index/null/typesVersions shapes at 60 each, and the rest at 15. No shape places a `.js`, `.jsx`, `.mjs`, `.cjs`, `.json`, `.mts`, `.cts`, `.d.mts`, `.d.cts` or directory competitor at the probed stem. The "zero false absence" claim therefore holds only where no other TS-admissible file exists. G1 shows exactly that hole. The JS-secondary guard itself is witnessed by a single sol case.
2. **Several axes are inert or coupled:**
   - `.mts`/`.cts` writers are 2/7 of the product and are Unsupported by construction (most of the 1,651).
   - `yarn` duplicates `npm`, because binding is link-based.
   - Package type is coupled writer = dependency, as R1 discloses.
   - Options are fixed: always `allowJs:true` and `jsx`, never `resolveJsonModule`, `allowJs:false` or `customConditions`.
3. **Paths are not crossed with mode or writer extension** (G3).
4. **The exports key grammar** covers string, conditions, one subpath, one pattern, null and array. It has no folder keys (G2), patterns competing with literals, or nested condition objects inside patterns.

A minor point: the ProvenUnresolved check consults only `resolveModuleName`, not checker ambient binding. My three ambient probes (an indexed `types/shim.d.ts`, and `node_modules/@types/shim` both shadowing a resolvable package and declaring a missing one) all returned `Unsupported("ambient module authority")`, so I found no defect there.

**Fix:** add an absence-falsification axis. For every shape/mode that yields ProvenUnresolved, generate variants that add each TS-admissible competitor at each probed candidate:
- JS family;
- `.json`;
- `.mts`/`.cts` and declaration variants;
- a directory with `index.ts`;
- a folder-mapping key;
- a case-variant spelling.

Also cross the paths/colon cases with all four modes × writer extension × scope type, and split writer type from dependency type. Report the effective independent-case count (excluding inert rows) beside the raw total. This closes the extension-table and usage-mode classes by measurement instead of by review enumeration.

**Self-critique:** a larger matrix is never a proof. The point is that the S2-critical label is currently certified by fixtures that cannot contradict it.

### S2. IMPLEMENTOR composition still describes patches, not the committed heads (SMELL, IMMATERIAL)

IMPLEMENTOR tells the controller to prepare from `92c1d0bc` and apply `R1-src.patch`/`R1-docs.patch`. It never names `03fa9c29` (proto) and `fa3bcb02` (plan) as the committed bases. I verified that the patch reproduces `03fa9c29` exactly, so the dispatch is unambiguous in effect.

**Fix:** state "source = `origin/proto/workspace-package-resolution` `03fa9c29`; packet = `fa3bcb02`", keep the patches as custody, and put the G1–G3 repairs and S1 axes ahead of the JS-secondary increment.

---

## Priority answers

1. **Round-1 WRONGs:** all fixed and reproduced against the oracle (table above). No inherited WRONG is silently downgraded.
2. **ProvenUnresolved boundary:** **not clean.**
   - G1: 7 cases where PKG says ProvenUnresolved and TS binds JSON.
   - G2: 2 cases where TS binds TS source through a folder mapping.
   - No Unsupported → ProvenUnresolved misfiling was found in the other probes. These cases correctly return Unsupported: case-variant target and specifier, a directory symlink inside the package, a gitignored `dist/index.js`, a hidden `.gen/` target, and the three ambient cases. A gitignored `.ts` target is Bound correctly.
3. **Gate adequacy:**
   - Oracle options and mode: correct.
   - Rerunnable: yes; I reproduced exact totals.
   - Vacuity: partial. The absence direction lacks competitor fixtures, and paths are not mode-crossed (S1).
   - Coverage of round-1 classes: the gate covers the round-1 failure *instances*, not the classes.
4. **Regressions introduced by R1:** G3 (colon paths in modern ESM). The cache spot check passed: outer package-scope bytes invalidate a warm cache. CPG/nav epochs are 108/64. Lane-P interaction is otherwise as SPEC §0.5 describes: `node10`/`bundler` colon paths agree with TS, and a matched-but-unproved paths key is Unsupported.
5. **IMPLEMENTOR:** usable. The stale wording is S2 (IMMATERIAL).

## Convergence classification (for the controller)

All three material WRONGs are **closed instances of round-1 families, each with a bounded fix.** None is a new family.

| Finding | Family (round 1) | Bounded fix |
|---|---|---|
| G1 JSON secondary pass | Extension-table fidelity (F1/F2), now in the absence direction | Add `.json` to the secondary-pass vector (one arm) plus 7 witnesses |
| G2 folder-mapping keys | Exports-search fidelity (sol W2/W6; my pattern-order check) | One Blocked predicate, or port `comparePatternKeys` with folder keys |
| G3 ESM colon paths | Usage-mode / EsmMode (F3/F4/W4), crossed with the scheme contract (F7), introduced by R1 | One `usage_mode` check in `resolve_in` plus 4 witnesses and 2 controls |

Round-over-round, the count fell from 7 WRONG mechanisms (plus sol's) to 3. All are smaller, and none repeats a fixed instance. Each is still another instance of "a TS resolution table not mirrored". That is why S1 matters: the closing move is the absence-falsification and mode-crossed axes, so that a third enumeration round is unnecessary.

## Verdict

**FIX.** The fixes are closed and enumerable:
- G1, G2 and G3, each with the RED witnesses listed. Each fails on `03fa9c29` under `differential-run.py`; G3 also fails against the `92c1d0bc` control;
- the S1 gate axes, so that ProvenUnresolved is falsifiable;
- the S2 wording.

No restart is warranted.

## Not checked

- Private F, CONTROLLER-pkg.sh, and the S2 scratch.
- The full MCP suite, mutgate, Tier-A and S1b. Only `js_packages_test::` was run (24/24).
- R1's 12-check cache script, which I did not rerun; I ran one outer-scope spot check.
- Public X/R/T stream identity, which I did not re-measure.
- Windows, Yarn PnP and Unicode aliasing. I probed only macOS case-insensitive aliasing.
- `allowJs:false`, `customConditions` values, `allowArbitraryExtensions` and module node18/node20. These are declined or conservative by inspection, not by oracle.
- Self-name × folder keys, and `#imports` folder keys (`#imports` is Unsupported anyway).
- I did not read the parallel sol round-2 review.

## Appendix: reproduction

The generator reuses `differential-generate.py`'s `emit()` verbatim (its head, up to `shapes={`). Writer `app.ts` imports `{real as picked}` and calls it; every package file is `export function real(){return 1;}`; tsconfig is `moduleResolution` M, `module` (M for node16/nodenext, else esnext), `allowJs`, `noLib`, `jsx: react-jsx`, plus listed options. Run `python3 probes/differential-run.py <dump-resolution> <typescript.js> <manifest> <out>`.

| ID | M | options / rootmeta | meta / files / rootfiles | spec |
|---|---|---|---|---|
| r2-json-bundler | bundler | resolveJsonModule true | exports `./data.json`; data.json | @ws/lib |
| r2-json-bundler-default | bundler | — | same | @ws/lib |
| r2-json-nodenext-cjs | nodenext | — | same | @ws/lib |
| r2-json-node10-main | node10 | resolveJsonModule true | main `data.json`; data.json | @ws/lib |
| r2-json-cond-fallthrough (control) | bundler | resolveJsonModule true | exports `{types:./data.json, default:./index.ts}`; data.json, index.ts | @ws/lib |
| r2b-json-legacy-subpath | node10 | resolveJsonModule true | data.json | @ws/lib/data.json |
| r2b-json-pattern | bundler | — | exports `{"./*":"./*.json"}`; data.json | @ws/lib/data |
| r2b-json-pkg-subpath | bundler | — | index.ts | @ws/lib/package.json |
| r2-trailing-slash | bundler | — | exports `{"./sub/":"./src/"}`; src/feature.ts | @ws/lib/sub/feature.js |
| r2-trailing-slash-nodenext | nodenext | — | same | same |
| r2-trailing-slash-vs-star | bundler | — | exports `{"./sub/":"./a/","./*":"./b/*"}`; a/x.ts, b/sub/x.ts | @ws/lib/sub/x.js |
| r2-paths-esm-colon | nodenext | root type module; paths `{"virtual:*":["./shim/*"]}` | rootfiles shim/pwa.ts | virtual:pwa |
| r2-paths-esm-colon-dir | nodenext | root type module; paths `{"virtual:x":["./shim/x"]}` | shim/x/index.ts | virtual:x |
| r2-paths-mjs-colon | nodenext | writer app.mjs; paths as above | shim/pwa.ts | virtual:pwa |
| r2-paths-esm-node-colon | node16 | root type module; paths `{"node:url":["./shim/url"]}` | shim/url.ts | node:url |
| r2-paths-esm-colon-js (control) | nodenext | root type module; paths `./shim/*.js` | shim/pwa.ts | virtual:pwa |
| r2b-paths-bundler-colon-dir (control) | bundler | paths `{"virtual:x":["./shim/x"]}` | shim/x/index.ts | virtual:x |
| r2b-paths-node16-cjs-colon (control) | node16 | paths `virtual:*` | shim/pwa.ts | virtual:pwa |
| r2-case-target / r2-case-spec | bundler | — | exports `./Index.ts` + index.ts / spec `@WS/lib` | — (Unsupported, agrees conservatively) |
| r2-ambient-* | bundler | — | `declare module "@ws/lib"` in node_modules/@types/shim or types/shim.d.ts | — (Unsupported) |
| r2-symlink-dir | bundler | — | exports `./src/index.ts`, packages/lib/src → ../../shared | — (Unsupported) |
| r2b-gitignored-js / -ts, r2b-hidden-ts | bundler | git init + .gitignore | dist/index.js; gen/index.ts; .gen/index.ts | — (Unsupported / Bound correct / Unsupported) |
