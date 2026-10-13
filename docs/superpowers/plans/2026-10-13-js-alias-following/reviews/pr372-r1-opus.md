# Review of slice L (PR #372, head `3dd5b7db`): follow `const` aliases to the callable they name

Reviewer: Opus 5.5 (independent). Round 1 of at most 2.
Goal judged against: "the nearest binding decides, and an alias is followed to what it names" (the positive half only).

**Verdict: FIX.** The binder and the three resolver routes are sound on every input I could construct; I found no binder-level wrong edge. Two things need to change before merge: (1) the edges this slice adds are missing from the incoming navigation index, and where main's textual path used to cover the site the branch loses a caller main reported; (2) a followed alias whose terminal the resolver cannot resolve drops a correct edge main had, on two ordinary shapes that no fixture pins.

## How I checked

- Read the full diff (`git diff origin/main...HEAD`) and the binder files it relies on.
- Ran the two prebuilt binaries (`L2-prism` = `3dd5b7db`, `main-38a59994-prism`; both report their commit in `--version`) on 22 synthetic fixture repos under `opus-scratch/fx/`, comparing every `call_site` record (`opus-scratch/run.py <fixture dir>`), and `nav callers` / `nav callees` with `--no-cache` (`opus-scratch/callers.py <fixture dir> callers|callees <symbol>`).
- Re-ran Excalidraw (`0642e72c`) with both binaries: 19,219 records on each side, none added or removed, exactly one site changed (`flush(0)` in `checkUnflushedContent` now binds `flush_NO_FILESYSTEM`, `local_def`, Exact). This matches the X column of the brief.
- Ran, from the test binaries already built in the worktree at the head (no rebuild, bless variable unset): 15 `js_binding_alias` unit tests, 29 `js_alias_follow` integration tests, 23 `js_alias_matrix` tests. All pass. The worktree is unchanged.
- Recomputed the matrix diff from `expected.jsonl` on `origin/main` and on the head.

Everything below marked "main" and "L2" is output I produced with those two binaries.

---

## WRONG findings

### W1. Followed-alias edges are missing from the incoming index; a caller main reported is lost — WRONG, MATERIAL

**Input** (`fx/a1`; the same shape as pinned matrix rows `local.ident-own-name-import.fn.call.fresh.*` and `local.ident-same-file-fn.fn.call.fresh.*`):

```ts
// m.ts
export function f(): number { return 1; }
// a.ts
import { f } from "./m";
export function viaImportAlias(): number {
  const h = f;
  return h();
}
```

| Query | main | L2 |
|---|---|---|
| `nav callers --symbol f` | `direct`, **`viaImportAlias`** | `direct` only |
| `nav callees --symbol viaImportAlias` | `f` (plus an unresolved `h`) | `f` |
| `--format callers` on a diff touching `f` (`fx/co`, same-file variant) | `direct`, **`viaAlias`** | `direct` only |

The same loss reproduces on the two pinned matrix fixtures I materialized: `nav callers f` and `nav callers local1` list `caller` on main and nothing on L2. The matrix scores those rows `CORRECT → CORRECT (field only)`.

The edges the slice newly adds are equally invisible to `nav callers`. On Excalidraw, the one gained edge shows in `nav callees --symbol checkUnflushedContent` on L2, and `nav callers --symbol flush_NO_FILESYSTEM` returns no caller on L2 (as on main). In `fx/d1`, `callers impl` and `callers dflt` are empty on L2 although `call-stats` shows the Exact edges.

**Mechanism.** Two things combine:

1. `src/navigation/mod.rs:358-394` adds an incoming edge only `count` times, where `count` comes from `scoped_caller_site_match_count` (`src/navigation/call_resolve.rs:29-80`). That function counts a site when its `callers` bucket key equals the target's name, or when an import binding whose local is the bucket key maps to the target. An alias site is filed under the alias spelling (`h`), which is neither, so the count is 0. Outgoing edges (`mod.rs:399-428`) are not gated, which is why `nav callees` is right.
2. On main the textual path filed a second record under the target's name (`f`), which passed the gate. The stand-down (`src/call_graph.rs:2762-2768`) removes that record.

`--format callers` (`src/output/review.rs:394`) walks `call_graph.callers` by name and loses the caller for the same reason.

The control that separates the gate from other causes (`fx/v1`): `const { plain } = N; plain()` is listed by `nav callers plain` on L2, and `const { plain: p2 } = N; p2()` in the same file is not. The only difference is whether the alias is spelled like the target. `--no-cache` rules out a stale sidecar.

**Why material.** The slice's value is the edges for followed aliases. In the query direction most navigation users ask ("who calls this"), those edges are absent unless the alias happens to be spelled like its target, and the simplest function-local alias regresses against main. The test named `one_followed_alias_call_is_one_edge_for_every_consumer` (`tests/integration/js_alias_follow_test.rs:483`) checks CPG `Call` edges and `resolved_caller_edges` only.

**Recommended fix.** In `build_resolved_call_edges`, treat a binder-proven site as already precise, as the code already does for `FuncValueField`: when `site.local_binding` is `AliasCallable`, `AliasImport` or `AliasNamespaceMember` (and, for the same reason, `Callable`), use `max(count, 1)`. Add a test that asserts `nav callers` of the target lists the alias caller for the import and same-file cases with a differently spelled alias, and a lane mutant that removes the bypass. For `--format callers`, either walk resolved edges for JS/TS targets or state the regression in the PR; I would fix it in the same change if it is a few lines.

**Alternative.** Keep a name-keyed record for followed aliases by filing the `Source` site under a second `callers` key (the terminal's name). Cost: a site would appear in two buckets, which every `callers` consumer and the merge and clear paths would have to tolerate; it also does not cover renamed exports. A cheaper non-fix is to not stand down in this slice; cost: the duplicate edge returns and the 39 `WRONG → CORRECT` rows that depend on the stand-down would carry a second, wrong record.

**Would not apply** to consumers that read `call-stats`, CPG `Call`/`Return` edges, `resolved_caller_edges` (membrane, echo, barrier) or `nav callees`: those are correct on L2. It also does not apply when the alias is spelled like its target.

### W2. A followed alias whose terminal cannot be resolved drops a correct edge main had — WRONG, MATERIAL

This answers question 2: yes, there are inputs.

**Input** (`fx/l1`; `tsconfig.json` maps `@/*` to `src/*`):

```tsx
import * as api from "@/api";            // src/api/index.ts exports fetchUser
import * as C from "../api/cjs";         // exports.legacy = function legacy() {}
export function Comp() {
  const { fetchUser } = api;
  const legacy = C.legacy;
  const onClick = () => { fetchUser(); legacy(); };
  function onHover() { return fetchUser(); }
  return [onClick, onHover];
}
```

| Site | main | L2 |
|---|---|---|
| `fetchUser()` in `onClick`, in `onHover` | Exact `free_single` → `src/api/index.ts:fetchUser` (the true callee) | dropped `ImportExternal` |
| `legacy()` in `onClick` | Exact `free_single` → `src/api/cjs.js:legacy` (the true callee) | dropped `UnknownName` |

In the same fixture `const save = saveUser` (a named import from `@/api`) resolves Exact on L2, and the relative twin `import * as rel from "../api/index"; const { fetchUser } = rel` resolves Exact, so only the namespace-member terminal over a non-relative in-repo module, and over a CommonJS `exports.x` module, is lost.

Main's edge was right by spelling (one function of that name in the repo), not by proof. The branch has more proof than main here (it knows the alias names member `fetchUser` of `@/api`) and emits nothing, because `js_ts_alias_member_outcome` accepts relative specifiers only (`src/resolution_js_alias.rs:71-79`) and the per-export table has no entry for `exports.x` (`:80-86`). A namespace destructure is same-named by construction, so main's name rung hits whenever the site sits in a nested named function and the name is unique.

The brief's "any site resolved → dropped: 0" is true for X (I reproduced it) and I take it as reported for T and F. It is not true in general, and no matrix row covers either shape: the only non-relative namespace rows are `external-ns-member-alias-trap`, whose truth is `none`.

**Recommended fix.** Resolve a non-relative namespace member through the same tsconfig-paths projection the named import uses (`js_ts_path_modules` / `js_ts_path_exports`, with its span requirement). `apply_js_paths` (`src/call_graph.rs:2228`) currently registers specifiers from member imports only, so namespace-import specifiers have to be added there. Packages stay `ImportExternal`.

**Alternative.** Keep the contract as is, add matrix fixtures for the two shapes (path-alias namespace member; CommonJS `exports.x` namespace member; `outer` scope, `same` naming), pin them with ledger entries, and say in the PR that a followed alias with an unresolvable terminal can lose a by-name edge. Cost: a known recall gap in the target population (a React app with path aliases), carried into the refusal slice. Falling back to the name rungs when the terminal is unresolvable is a third option; I would not take it, because it re-opens the by-name edge for real packages (`import * as P from 'lodash'; const { map } = P`).

**Would not apply** if the owner accepts that by-name edges for aliases are going away in the next slice regardless; then only the fixtures and the disclosure are needed. Measure the yield of the paths route on F before choosing.

### W3. The `CLAUDE.md` sentence is false at this head — WRONG, IMMATERIAL

`CLAUDE.md:246-248`: "A JS/TS local alias resolves Exact only when a bounded chain of unique, unwritten `const` bindings reaches one callable, an import, or a namespace-import member; spelling alone is not binding evidence."

**Input** (`fx/doc1`), identical on main and L2:

```ts
import { f, g } from "./m";
function h(): number { return 0; }
export function viaLet() { let h = f; return h(); }          // Exact local_def → a.ts:h
export function viaConditional(x: boolean) {
  const pick = x ? f : g;
  return function inner() { return pick(); };                 // Exact free_single → other.ts:pick
}
```

Both are local aliases resolved Exact by spelling. At the head the matrix still pins 94 `WRONG` rows: 56 through `free_single`, 30 through `local_def`, 8 through the textual path (recomputed from `expected.jsonl`; it matches the brief). Most are local-alias shapes the slice does not follow (`let-stable`, `var`, `conditional`, `shadows-same-file-fn-let`, and others). The sentence describes the state after the refusal slice.

**Recommended fix.** Reword to what this slice does: "A JS/TS `const` alias chain the binder follows (bounded, unique, unwritten links, one file) resolves through the callable, import or namespace-import member it names and never by spelling. An alias the binder does not follow still keeps base behavior." Let the refusal slice change the second sentence. **Alternative:** leave it and merge the two slices together; cost: `main` carries a false contract statement if the second slice slips. Also split the line; it was appended to an existing one.

**Would not apply** once the refusal slice lands.

### W4. A span-less export binds a nested same-named function, now also from alias sites — WRONG, IMMATERIAL

**Input** (`fx/e2`):

```ts
// x.ts
declare function compute(): any;
let w: any;
w = compute();
export { w };
export function outer() { function w() { return 7; } return w; }
// a.ts
import { w } from "./x";  import * as N from "./x";
w();                                  // main and L2: Exact import_member → x.ts:w@6 (the nested function)
const a = w; a();                     // main: dropped.  L2: Exact → x.ts:w@6
const b = N.w; b();                   // main: dropped.  L2: Exact → x.ts:w@6
const { w: c } = N; c();              // main: dropped.  L2: Exact → x.ts:w@6
```

The exported `w` is the module-level `let`; the nested function is unrelated. The cause predates this PR: a written or may-call export is recorded without a span, and the import route then accepts any non-method function of that name in the file. The direct import and the qualified call have the same wrong edge on main. This PR makes it reachable from three more site shapes, and `js_ts_alias_member_outcome` re-implements the span-less acceptance (`src/resolution_js_alias.rs:98-102`, `is_none_or`).

Immaterial to this slice's goal because the alias is followed correctly and resolves exactly as its direct twin does, which is the stated contract.

**Recommended fix.** Fix it at the root in a separate change: a span-less local export should bind only a module-scope function. **Alternative in this PR:** require `resolved.span.is_some()` on the two alias routes, following the precedent the paths route sets ("New alias authority requires a source-backed callable span", `src/resolution.rs:3818-3822`). Cost: alias sites lose the may-call-export edges their direct twins keep; measure the yield on the matrix and F first.

**Would not apply** when the file's only function of that name is the export's own inline function (the accepted may-call ruling).

### W5. The PR description mislabels the field-only rows — WRONG, IMMATERIAL

The description (and the brief) say the 27 field-only rows include "kind `import_qualified` → `import_member`". No changed row has `import_qualified` on either side. The 27 are: 19 rows `free_single` → `import_member`, 6 rows where the `IndirectResolution` record disappears, 2 rows where the drop reason goes `UnknownName` → `ImportExternal`. The difference matters to a reader: on main those 19 sites were right through the repo-wide name search. The description also lists "nested or rest destructuring" as not followed; a rest or nested sibling is followed (`const { f: h, ...rest } = N`, `const { f: h, g: { length } } = N`; both Exact on L2 in `fx/b1`, both correct), and only a name bound by the rest or nested part is not.

**Fix:** correct the two lines. **Alternative:** none needed.

---

## SMELL findings

**S1. Test and lane gaps — SMELL, IMMATERIAL.** No test or mutant covers the incoming index (W1). By reading, not by running, these behavior-changing mutants would survive the 34-mutant lane:
- removing `!self.method_owners.contains_key(*fid)` in `js_ts_alias_member_outcome` (it matters for a span-less export, where a same-named method would become the Exact target);
- `ids.len() == 1` → `!ids.is_empty()` there (matters only for span-less exports);
- dropping `AliasNamespaceMember` from the stand-down `matches!` (matters when a function is named like the namespace local: `fx/tx` shows the textual path emitting a `utils()` record for `const { fmt: h } = utils` on main).
The nested-sibling pattern has no unit row (the rest-sibling does). Fix: add a span-less-export row and a nested-sibling row; alternative: record them as known lane limits.

**S2. No telemetry for followed aliases — SMELL, IMMATERIAL.** A followed site leaves the `local_binding_may_call["alias"]` counter and is counted nowhere; its edge kind is `local_def` or `import_member`, the same as a direct call. Measuring this lane needs a full site-dump diff. Fix: a `call-stats` counter by terminal (callable, import, namespace member) and by outcome. Alternative: keep diffing dumps; cost is slower measurement in the next slices.

**S3. The visited set is unreachable as a distinct behavior — SMELL, IMMATERIAL.** The follow is deterministic in `(at, spelling)`, so a revisit can only loop, and the bound of 8 returns `None` for any loop. Either mechanism alone terminates the walk. Keep both if the visited set is meant as the stated guarantee and the bound as a cost limit; say so in the comment. Alternative: drop the set.

**S4. An undisclosed second effect: argument data-flow edges — SMELL, IMMATERIAL.** In `fx/t1` (`const h = sink; h(x)` with `x` from `req.query`), `--algorithm taint` reports nothing on main and one `taint_sink` warning on L2. Main's indirect record never bound arguments: the argument index is keyed by `(start_byte, callee name)` (`src/ast.rs:11829`), and that record carries the target's name (`sink`) at a site whose source callee is `h`. The `Source` record matches. This is correct and a gain, but it is a change in review output that the PR's "Effect" section, which counts call-site records, does not mention.

**S5. Direct twins are now weaker than the alias route — SMELL, IMMATERIAL.** `import { f }`, a block-local `f` elsewhere in the caller: the direct call `f()` outside the block is dropped by the flat caller-locals guard, and `const h = f; h()` at the same place resolves Exact and correctly (`fx/r1`, `c5` against `c4`). Replacing the flat guard with the binder's answer for direct imports is a follow-up.

---

## The two decisions

**Decision 1 (namespace member through the per-export proof): right. Do not mirror the qualified route.** `fx/d1`, `fx/d2`, `fx/s1`, with the qualified call identical on main and L2:

| Member of `import * as N from './m'` | `N.m()` (qualified route) | `const h = N.m; h()` on L2 | Truth |
|---|---|---|---|
| exported function | Exact | Exact, same target | same |
| `export { impl as renamed }` | dropped | Exact → `impl` | `impl` |
| default export (`N.default`) | dropped | Exact → the default function | that function |
| `export * from './star'`, and through a barrel | dropped `ImportExternal` | Exact | the star target |
| function declared in `./m` but not exported, same name also in `lib/m.ts` | **Exact to both** | dropped | none |
| name absent from `./m`, present in unrelated `lib/m.ts` | **Exact → `lib/m.ts`** | dropped | none |
| ambiguous star (two sources export it) | — | dropped (as the named import) | none |
| CommonJS `exports.cj = function` | Exact (stem match) | dropped | `cj` |
| `./x.jsx` naming `x.tsx` | Exact (sibling probe) | dropped `ImportExternal` (as the named import) | `f` |

Mirroring the qualified route would import its two false-Exact rows into alias sites and lose the three rows it cannot resolve (renamed, default, star and barrel). The two rows where the alias is weaker are gaps of the named-import route, which the alias matches by contract; fix them there. The asymmetry with `N.f()` should be closed by repairing the qualified route (#371), not by copying it.

**Decision 2 (the textual path stands down where the binder followed): right for the records, not yet safe for every consumer.**
- CPG `Call`/`Return` edges: safe. One edge per call; the pinned test covers it.
- Argument data-flow edges: safe, and better (S4).
- `resolved_caller_edges`: safe.
- `nav callees`: safe.
- `nav callers` (and everything on the incoming index) and `--format callers`: **not safe**; they lose the caller (W1). Fix the gate and the decision stands.

---

## Answers to the eight questions

1. **Wrong edges.** None from the binder. Probed with both binaries: parameter, catch, block, `for`-head, switch, class-body, namespace-body, label and hoisted-`var` shadowing of the alias and of the right-hand name; TDZ order; compound, update, destructuring (array, object, rest, default), `for`-`of` head and nested-closure writes to the alias, the terminal and the namespace; default, namespace and type-only imports; `import =` forms; star, ambiguous-star and barrel re-exports; JSX against call for wrapped values and intrinsic tags; sealed and unsealed parse errors; an ASI hazard; self and mutual cycles; minified one-line code with duplicate spans. Every outcome was the static binding or a drop. The one wrong Exact I found is W4, inherited from the import route.
2. **Lost edges.** Yes: W2 (correct by-name edges dropped) and W1 (a caller lost from the incoming index).
3. **Decisions.** Above.
4. **Bound, visited set, re-entrancy.** The loop either returns or increments `links`, so it runs at most nine iterations. Nothing the follow calls can call the follow: `js_ts_site_binding` → `js_ts_classify` → `js_ts_classify_call` → `js_ts_wrapper_provenance` does not classify, and `js_ts_namespace_binding` only calls `js_ts_site_binding`. Eight is the right kind of limit: a fixed cost bound that fails to "not followed". See S3 on the redundancy.
5. **Persisted state.** Complete. The variants are appended, `local_binding` is outside `cmp_key`, CPG cache 116 has a round-trip and a 115-miss test, the sidecar stores no new shape and is bumped to 64, and the two other lanes' version mutants were updated. The follow is file-local, so a partial cache hit keeps valid bindings and resolution is recomputed.
6. **Tests and mutants.** By reading the two test files against the contract, each conjunct has a test that fails without it, except the ones in S1. I agree all three judged-equivalent mutants are equivalent: the visited set (S3); the `lexical_declaration` parent check (a `variable_declarator` whose parent's first child is `const` always has that parent); `(Import, Some)` against `(_, Some)` (`js_ts_namespace_binding` returns a module only when the binding is `Import`).
7. **Matrix diff.** 307 rows: 239 + 39 + 2 + 27, recomputed. No changed row is wrong and every row in a followed shape is `CORRECT` or `NO-RECORD` (`new` forms) at the head. Six of the "field only" rows hide the W1 regression, and the matrix has no row for either W2 shape. The field-only description is mislabeled (W5).
8. **Docs.** Not accurate at this head (W3).

---

## Follow-ups: wrong on main, unchanged by this PR

- `function c() { const { f } = require('./b'); f(); }` binds Exact `local_def` to two unrelated block-local arrows named `f` (`fx/r1`, `c2`).
- The qualified route's stem fallback: `N.hidden()` and `N.onlyInLib()` Exact to non-exported or unrelated-file functions (`fx/d1`; #371 item 1).
- `const g = function named() {}; g()` resolves Exact, and `nav callers named` is empty (`fx/n1`): the same gate as W1, fixed by the same change.
- The span-less export fallback for the direct named import and the qualified call (root of W4).
- `import f = N.g; f()` is `name_only` to every function named `f` (`fx/ie`).

---

## Verdict

**FIX.**

Must change before merge:
1. **W1**: index incoming edges for binder-followed alias sites, with a `nav callers` test and a mutant; fix or disclose `--format callers`; rename or extend the "every consumer" test.
2. **W2**: resolve path-alias namespace members through the paths projection, or pin both shapes in the matrix with ledger entries and correct the "resolved → dropped: 0" claim to "on the three corpora".

Should change, not blocking: W3 (one sentence), W5 (two lines of the description), S4 (one line of the description). W4, S1, S2, S3, S5 are recorded for follow-up.

All findings are closed and enumerable with bounded fixes on this PR; one re-review round should settle them.

### Checked and found sound

- The binder: which declarations are followed, the three link shapes, value unwrapping, the lookup of each right-hand name at its own position, the namespace proof at the qualifier, the bound, termination and non-re-entrancy.
- Site mapping to the three new variants; the `None`-qualifier arm ordering; the R4c refactor (behavior-preserving for Python and for direct JS/TS imports).
- `AliasCallable` (by name and span, wrapped from JSX only), `AliasImport` (relative and tsconfig-paths, without the spelling guard), `AliasNamespaceMember` (per-export, relative only), and that none falls to a name rung.
- Decision 1 against the qualified route on nine member kinds.
- The stand-down for CPG edges, argument data-flow, `resolved_caller_edges` and `nav callees`; the textual path unchanged where the binder does not follow.
- Cache versions and the round-trip and version-miss tests.
- The matrix diff counts, the 94 remaining `WRONG` rows and their 56 / 30 / 8 split, the two `WRONG → MISSING` rows.
- The X column of the measured table.

### Not checked

- The T and F columns. I did not open F, and did not re-run T.
- The mutation lane: not executed. The survivors in S1 are predictions from reading.
- The full test suite locally. I ran 67 tests from prebuilt binaries and rely on CI for the rest.
- The MCP tools and `nav ego`. They read the same incoming index as `nav callers`; I did not exercise them.
- `--format callers` across files: my TypeScript cross-file probe returned no callers on both binaries, so it showed nothing; the same-file JavaScript probe is the evidence.
- Ground truth by the TypeScript checker for my fixtures. Truth is by reading.
- The partial-hit incremental cache path (read, not exercised), performance, and how the stacked refusal slice interacts with any of this (I only confirmed it does not touch the navigation gate or `review.rs`).

**Verdict: FIX** (W1 and W2 before merge).
