# Review of PR #372, round 2 of 2 (confirmation): follow `const` aliases to the callable they name

Reviewer: Opus 5.5 (independent). Head `d22f8349`, base main `38a59994` (merge base `86756aad`).
Goal judged against: "the nearest binding decides, and an alias is followed to what it names", with the tightened promise: at every site either (1) the alias is followed and its terminal is proven, giving one Exact target, or (2) exactly main's records.

**Verdict: FIX.** One material item remains, with a one-line repair, and it was introduced by the fold: a followed alias of an import whose export has no span (the CommonJS export forms that bind a named function, and a written `let` export) loses a correct edge main had. Apart from that, the round-1 findings are closed and I found no new wrong edge on any input I could construct. The remaining items are closed and enumerable; no third review round is needed.

## How I checked

- Read the whole-PR diff and the fold diff (`3dd5b7db..HEAD`), and every reader of `CallSite.local_binding` in `src`.
- Ran the prebuilt binaries on synthetic fixtures under `r2-opus-scratch/fx/` (`run.py <dir>`; three-way with the pre-fold head `L2-prism` as the attribution control: `run3.py <dir>`). `L3-prism --version` reports `d22f8349ec22`, main reports `38a599948771`.
- A generated sweep of 852 fixtures (`sweep_gen.py`, `sweep_run.py`, `sweep_cmp.py`): 15 export forms x import/link shape x placement (same function, outer function with a nested caller, module level) x call or JSX x with or without a decoy function named like the alias. Each fixture has a direct-call twin of the same import. Run on main, L2 and L3.
- Materialized all 1,335 matrix fixtures into scratch and compared every `call_site` record (all lines, not only the `// SITE` line) between the two binaries (`matrix_run.py`).
- Re-ran both binaries on Excalidraw `0642e72c` with fresh caches; rescored the saved T dumps.
- In the worktree, with the bless variable unset: `cargo test --lib js_binding_alias` 17 passed, 0 failed; `cargo test --test integration js_alias` 71 passed, 0 failed (includes the matrix). `git status` in the worktree is empty before and after. `gh pr checks 372`: nine checks pass.

"main", "L2" and "L3" below are output I produced with those binaries.

---

## WRONG findings

### W1. A followed alias of an import whose export has no span loses a correct edge main had (introduced by the fold) — WRONG, MATERIAL

**Input** (`fx/q2a`; sweep id `cjs_obj.ident.same_fn.call.nodecoy`):

```js
// lib_cjs_obj.js
function helper() { return 1; }
module.exports = { helper };
// app_same.js
import { helper } from './lib_cjs_obj';
export function direct() { return helper(); }
export function viaAliasSameFn() {
  const h1 = helper;
  return h1();
}
```

| | main `38a59994` | L2 (pre-fold `3dd5b7db`) | L3 (head `d22f8349`) |
|---|---|---|---|
| `helper()` in `direct` | Exact `import_member` -> `lib_cjs_obj.js:helper` | same | same |
| records at `h1()` | `h1` Source: dropped `UnknownName`; **`helper` IndirectResolution: Exact `import_member` -> `lib_cjs_obj.js:helper`** | `h1` Source: Exact -> `helper` | `h1` Source: dropped `UnknownName`; nothing else |
| `nav callers --symbol helper` | `direct`, **`viaAliasSameFn`** | not run | `direct` only |
| `nav callees --symbol viaAliasSameFn` | resolves `helper` | not run | no resolved callee at that site |

The same loss reproduces for `exports.named = named`, `module.exports.f = f`, `module.exports = f` with a default import, a re-export of a CommonJS export (`export { f } from './cjs'`), a written export (`export let f = function f() {}; f = ...`), and for the JSX element form (`fx/q2a`, `fx/q2e`, sweep). In each, the direct call of the same import resolves Exact to that function on both binaries, so the edge main had is the one prism's own direct route gives.

Sweep, site records against main, per export form (60 fixture variants each, 36 for the two default-import forms):

| Export form | L2 (pre-fold) | L3 (folded head) |
|---|---|---|
| `module.exports = { target }` | 60 proven | 58 same, **2 lost** |
| `exports.target = target` | 60 proven | 58 same, **2 lost** |
| `module.exports = target` (default import) | 36 proven | 34 same, **2 lost** |
| written `export let target = function target() {}` | 60 proven | 58 same, **2 lost** |
| span-less export with a nested same-named decoy (the disclosed shape) | 60 proven (wrong, round-1 Opus W4) | 58 same, 2 lost (main's edge there is the decoy) |
| ESM function, arrow, rename, re-export, star, default; wrapped export at JSX sites | proven | proven, identical to L2 |
| exported alias, missing export, `exports.target = function () {}`, wrapped export at call sites | 35 lost in total (the round-1 findings), 25 drop-reason changes | same as main, apart from 2 removed dropped records |

So the fold removed L2's 35 lost sites (all were main's by-name decoy edges, which the promise now keeps) and created 10 lost sites in the same-function placement. In all 10 the lost target is what the direct call of that import resolves to; in 8 it is the function the export declares, and the other 2 are the disclosed nested-decoy shape.

**Mechanism.** Two fold decisions combine, exactly where question 2 points:
1. The import alias route is proven only by a span (`src/resolution.rs:3811-3815`), so these sites are "not proven" and the Source record falls through to the name rungs for the alias's own spelling, which drop.
2. The textual path stands down on the binding kind (`src/call_graph.rs:2762-2768`), not on the route's outcome. On main that path filed the record for `helper`, which resolved through the direct import route including its span-less fallback. The head removes the record whether or not the alias route produced an edge.

All four conditions are needed: an alias of an import under its own name (the textual path needs the right-hand name to be a function name); an export recorded without a span; alias and call in the same function; no function anywhere in the repo named like the alias (main skips the textual path otherwise).

A second, smaller effect of the same mechanism: where main's textual record was itself dropped, the head has one record fewer and no edge difference (`fx/q2c`: `import { map } from "lodash"; const h = map; h()` with a function named `map` elsewhere; `const { plainFn: h } = utils` over an unresolvable namespace, with a function named `utils` elsewhere; sweep `cjs_prop_inline.ident.same_fn.*.nodecoy`). The corpus table's "record added or removed: 0" is true of X, T and F and is not true in general.

**Why material.** The promise was tightened in round 1 because both reviewers showed edges main had being lost. This is the same harm, in the population the slice targets, and it did not exist before the fold. The PR description and the brief describe this exception as removing "a wrong edge to a nested same-named function"; that is true of the one fixture pinned (`an_export_without_a_span_is_not_a_proven_alias_terminal`) and false for CommonJS exports, where the removed edge is the right one. No matrix row covers the shape (`spanless-export-nested-decoy` uses `N.w`, which never had a textual record). The three corpora are ESM TypeScript, which is why they show zero.

**Recommended fix.** Stand down only where the alias route produced the site's edge. In `compute_indirect_call_sites`:

```rust
let alias_followed = self.js_ts_alias_route_hit(site);
```

The route is decidable there: `recompute_indirect_calls` runs after `apply_js_paths` and `apply_js_export_resolution` in both the full build (`src/call_graph.rs:2102-2106`) and the cached build (`src/cpg/build.rs:762-767`). With this, a not-proven import or namespace-member site has main's records by construction, the dropped-record difference goes away, the "one disclosed difference" paragraph can be deleted (main's two records return for the nested-decoy shape), and `nav callers` and `--format callers` list the caller again through the record filed under the target's name. Then:
- change the named-import expectation in `an_export_without_a_span_is_not_a_proven_alias_terminal` to main's two records;
- add a fallback test for `module.exports = { target }`, `exports.target = target` and `module.exports = target`, call and JSX, asserting main's records, and a `nav callers` assertion for one of them;
- add matrix rows for those shapes (truth `one`);
- re-anchor mutants AF-34, AF-35 and AF-41, and add one that keys the stand-down on the binding kind again.

For a callable terminal that does not hit (W2), that expression also brings main's textual record back. If the callable terminal should keep standing down whatever the outcome, write `matches!(site.local_binding, JsLocalBinding::AliasCallable(_)) || self.js_ts_alias_route_hit(site)`; either is consistent as long as the description says which.

I did not build or run this change.

**Alternative.** Keep the code, correct the description to say the stand-down can remove a right edge for span-less exports, and pin the CommonJS shapes with ledger entries. Cost: a known regression against main ships, and the promise needs a second exception. Dropping the span requirement on the import alias route is a third option; I would not take it, because it re-opens round-1 Opus W4 for identifier aliases in every placement.

**Would not apply** if the owner decides the textual path is retired for followed aliases regardless of outcome; then it is an accepted regression to disclose and pin. It also does not apply when the alias is declared outside the calling function, or when a function named like the alias exists (main has no textual record there).

### W2. A followed alias whose in-file callable terminal cannot bind refuses the name search — WRONG against the stated promise, IMMATERIAL

**Input** (`fx/q2b/wrapped_nested.jsx`, with `other.js` exporting `function helper`):

```jsx
const W = memo(function Inner() { return null; });
export function outer() {
  const helper = W;
  return function run() { return helper(); };
}
```

| Site | main | L3 (and L2) |
|---|---|---|
| `helper()` in `run` | Exact `free_single` -> `other.js:helper` | dropped `WrappedExportNonJsx` |
| `h()` after `const h = W2;` in the same function (`wrapped_same.jsx`) | Source dropped `UnknownName`; IndirectResolution Exact `local_def` -> `W2` | Source dropped `WrappedExportNonJsx`; no second record |
| one-line file with two same-named functions on one line, alias in an outer function (`minified_nested.js`) | Exact `free_single` -> `other.js:helper` | dropped `UnknownName` |
| same, alias and call in one function (`fx/q2c/src/minified.js`) | IndirectResolution Exact to both `f@1` (one is the callee) | no record with a target |

`js_ts_alias_route` returns `Some(outcome)` for a callable terminal whatever the outcome (`src/resolution_js_alias.rs:46-48`), so the site does not continue as main does. The code comment says so and `call-stats` counts it (`callable_unresolved`), but the brief's handling table ("returns a proven hit or `None`") and the PR description ("It never refuses a name search") do not. The import and namespace-member terminals behave differently for the same value: a wrapped import called as a function falls through to main's name search.

Immaterial because every edge removed in the first three rows is one prism's own model calls wrong (a `memo`/`forwardRef` value called as a function; a by-name decoy), and the fourth needs two same-named functions on one line. It is unchanged since round 1.

**Recommended fix.** State the exception where the promise is stated (PR description, the `CLAUDE.md` sentence): an alias whose terminal is an in-file callable resolves exactly as a direct use of that callable does, including its drops. Add one fallback-test row that pins it. **Alternative:** return `None` unless the callable route hits, for exact parity. Cost: main's wrong edges return at wrapped-call sites, and `a_wrapped_terminal_binds_from_jsx_only` changes.

**Would not apply** once the refusal slice lands.

### W3. The fall-through reads the alias's import proof — WRONG, IMMATERIAL

**Input** (`fx/q2d/src/reentrant.tsx`; `@/comp` is a tsconfig-paths module exporting `F = memo(...)`):

```tsx
import { F } from "@/comp";
const g = F;
export function outer() {
  const F = g;
  return function inner() { return F(); };
}
```

`F()`: main dropped `UnknownName`; L3 dropped `WrappedExportNonJsx`, counted as `import_kept_main`. The alias route returns `None` (wrapped, not JSX); the site falls to R4c, finds the import binding spelled like the site, and `import_proven` is true there because it compares `AliasImport.local` with the binding (`src/resolution.rs:3768`) without asking which route is calling. Main's `MayCall("alias")` never passed that check. It needs an alias re-declared under the import's own name in an inner scope. Only the drop reason can differ: by reading, a non-relative import yields at most one span-backed candidate, and exactly one is a proven hit on the alias route before the fall-through is reached.

**Fix:** `AliasImport { local } => alias_route && *local == binding.local`, and re-anchor AF-25. **Alternative:** leave it and note it; cost: the `kept_main` counter keeps a case that is not main's record. **Would not apply** to relative modules, where `import_proven` is not consulted.

---

## SMELL findings

**S1. `nav taint-reaches` (and the MCP `taint_reaches` tool) does not descend through a followed alias unless it is spelled like its target — SMELL, IMMATERIAL.** This is the answer to question 5. `descend_target` and `is_parameter_binding_from` (`src/cpg/trace.rs:641-773`) look the call site up in `call_graph.callers[<target function name>]`. Measured (`fx/taint`, `fx/taint2`, `fx/t1`):

| Call | main | L3 |
|---|---|---|
| `sink(x)` | `Reached` | `Reached` |
| `const { sink } = N; sink(z)` | `NotReached` | `Reached` |
| `const h = sink; h(y)` | `NotReached` | `BoundaryExited` |
| `import { sink as renamed }; renamed(x)` (no alias) | `BoundaryExited` | `BoundaryExited` |

On the same fixture `--algorithm taint` reports the sink reach on L3 (nothing on main), so the two taint consumers disagree at one site. It fails closed and is better than main's `NotReached`; the last row shows the gate has the same limit for a renamed import on main. Fix: add it to the description's "still name-keyed" list and record a follow-up to key the gate on the site. Alternative: fix the gate in this PR (match the site by caller and span over `calls`, as `return_flow_ascent_allowed` already does); cost: a change to the reasoning layer outside this slice.

**S2. An import or namespace member whose export is CommonJS or a written `let` is never proven on the alias routes — SMELL, IMMATERIAL.** A consequence of the span requirement that neither the brief nor the description states: in the sweep, 216 sites that L2 resolved (204 to the direct twin's target; the twin is unresolved for the other 12) are no longer proven on L3. 208 of them equal main and 8 are the W1 losses. It is a yield cost, not a regression, once W1 is fixed. Fix: one sentence in the description; the lasting repair is the recorded follow-up on the direct routes' span-less edge. Alternative: none needed.

**S3. Small items — SMELL, IMMATERIAL.** The `kept_main` counter label includes sites whose records differ from main until W1 and W3 are fixed. The `listed` check in `to_callers_output` is redundant with `visited` (an equivalent-mutant candidate). The `CLAUDE.md` sentence is appended to an existing line.

---

## Answers to the five questions

**1. Closure.**

| Round-1 finding | State | Evidence |
|---|---|---|
| Pattern binding a name twice (sol W1) | Closed | `fx/w1`, `fx/w1b`: the four reported patterns plus numeric key, two plain keys, shorthand plus pair, default, array, two declarators and three escaped-identifier duplicates all equal main; valid flat patterns still follow |
| Unproven terminal dropped main's edge (sol S1, Opus W2) | Closed for the Source record; open where main's edge came from the textual record (W1) | `fx/w2`: all five round-1 sites equal main; sweep: L2's 35 lost sites are gone |
| Span-less export bound a nested function (Opus W4) | Closed | `fx/w4`: four alias sites no longer bind the decoy; direct routes unchanged |
| Callers index and `--format callers` (Opus W1) | Closed, apart from the caller W1 loses | `fx/callers`: import, same-file, member, shorthand and renamed aliases each listed once, two sites on one line twice; `fx/fc2`; warm cache equals cold |
| Test and mutant gaps (sol S3, Opus S1) | Closed by reading | the named tests and AF-31/35/41 exist; 41 lane entries; lane not executed |
| Counter (Opus S2) | Closed | `stats.py` on seven fixtures |
| `CLAUDE.md` "only" (sol S2, Opus W3) | Closed | accurate apart from W1 and W2 |
| Description (Opus W5, S4) | Closed for the round-1 points; three new inaccuracies | "never refuses a name search" (W2); the "one shape" paragraph (W1); taint statement (S1) |
| Follow-ups not changed | Agreed, not re-raised | |

**2. The promise.** Not true as stated. The complete list of differences that are not a proven hit: W1 (and its dropped-record variant), W2, W3. This is a closed population, not a failed search: the three alias variants are read in six places (the stand-down, the route, `import_proven`, the counters, `to_callers_output`, the callers index through `js_ts_alias_route_hit`), and every other `local_binding` match treats them as it treats `MayCall("alias")`. Independent confirmation: across all 1,335 matrix fixtures, 1,032 have identical record sets on both binaries and the other 303 are each one Source record with one Exact target on the head, with no off-site difference.

**3. The fold's own code.** Whole-pattern admission, the fall-through for import and namespace-member terminals, the span requirement on both routes, the once-only callers count (also when the alias is spelled like its target; non-alias sites untouched) and the `--format callers` change are sound. In the sweep every proven hit equals the direct-call twin's target or the twin is unresolved; none differs. The defects are the stand-down's key (W1), the callable terminal (W2) and `import_proven` (W3).

**4. New and material from the fold.** W1 only. S2 is new and immaterial.

**5. Other spelling-keyed consumers.** S1. One more that is identical on main: `compute_scope` Tier 1 (`src/cpg/context.rs:433-440`) reads the skeleton graph's name-keyed `callers`, which never held textual records.

---

## Follow-ups: identical on main, not made more reachable

- `compute_scope` Tier 1 does not pull in a file whose only call into a changed function is through a differently spelled alias.
- The `taint-reaches` descent gate misses a renamed direct import.
- `function g(N) { const { f } = N; f(); }` binds Exact `local_def` to an object-literal method `f` in another function of the file (`fx/pat`).

---

## Probe log

| Hypothesis | Prediction | Result |
|---|---|---|
| Stand-down on binding kind removes a right edge where the import route is span-less | main: textual Exact edge; L3: none; L2: Exact (fold-introduced) | Confirmed three ways (`fx/q2a`, `fx/q2e`, sweep) |
| Callable terminal that cannot bind drops instead of falling through | main: by-name edge; L3: drop | Confirmed (`fx/q2b`); same on L2 |
| `import_proven` leaks into the fall-through | main `UnknownName`, L3 `WrappedExportNonJsx` | First probe inadmissible (my `tsconfig.json` was not picked up: the direct call dropped on both); confirmed after fixing the fixture (`fx/q2d`) |
| Escaped-identifier duplicates bypass whole-pattern admission | L3 Exact where main drops | First probe inadmissible (the shell collapsed the escapes); with real escapes, refuted: all equal main |
| The taint descent gate is name-keyed | alias: no `Reached`; same-named alias: `Reached` | Confirmed |
| Binder and registry spans disagree for some callable kind | a callable terminal drops | Refuted for nine kinds (`fx/kinds`) |
| `--format callers` across files | lists the alias caller | Inadmissible: no caller listed on either binary, the direct caller included; the same-file probe is the evidence |

---

## Fix list (closed; no further review needed)

Before merge:
1. W1: key the textual stand-down on `js_ts_alias_route_hit(site)`; the tests, matrix rows and mutants listed under W1.
2. Description: delete or rewrite the "one shape" paragraph to match the outcome of item 1; state the callable-terminal exception (W2) next to "never refuses a name search"; add `nav taint-reaches` / `taint_reaches` to the name-keyed list (S1); say CommonJS and written-`let` exports are never proven on the alias routes (S2).

Should change, not blocking: W3 (one token and a mutant anchor). Recorded: S3 and the follow-ups.

### Checked and found sound

- Whole-pattern admission on more than 30 pattern variants, valid and invalid (`fx/w1`, `fx/w1b`, `fx/pat`).
- The import and namespace-member fall-through: the Source record equals main's on every not-proven shape in the sweep and the matrix.
- The span requirement on both alias routes; direct routes unchanged.
- The callers index count and `--format callers`; warm cache equals cold.
- The `js_alias_follow` counters.
- Matrix claims, recomputed from `expected.jsonl` and re-measured with both binaries: 1,169 main rows, 278 verdict moves (239 + 39), 25 field-only (19 + 6), no row leaving CORRECT, 166 new fixtures all identical to main.
- The X column, re-run: 19,219 records on each side, one site gained, nothing added, removed or otherwise changed. My fresh dumps match the saved ones.
- The T column, rescored from the saved dumps: 61,712 on each side, no change.
- Build ordering for the recommended fix, by reading.

### Not checked

- The F column. I opened nothing private.
- T was not re-run; I rescored `measure/T/base2.sites.jsonl` against `L3.sites.jsonl` and take `base2` to be main's output, as it is for X.
- The mutation lane was not executed.
- The full test suite locally: I ran 88 tests and rely on CI for the rest.
- The recommended fixes: not built, not run.
- MCP tools and `nav ego` (same index as `nav callers`); `--format callers` across files; the partial-hit cache path; the Tier-A harness.
- Truth by the TypeScript checker for my fixtures; truth is by reading and by the direct-call twin.
- The later slices stacked on this one.

**Verdict: FIX** (W1 before merge, with the description corrections; the list is closed and needs no further review round).
