# Review brief — slice L: follow `const` aliases to the callable they name (JS/TS)

You are reviewing as a senior engineer who will live with this resolver for years. Round 1 of at most 2.

## The goal this change serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves a JS/TS call `f()` or element `<F />` by asking what the name is bound to. On main, when the nearest binding is a local alias (`const f = imported;`, `const f = N.f;`, `const { f } = N;`) the binder answers "alias, keep base behavior", and base behavior never looks at what the alias names: the site is dropped, or resolved by spelling to a same-file function of the alias's name or to the one function in the repo with that name. An earlier attempt to refuse that by evidence rules (the parked "E5" slice) was reviewed twice and each round found alias shapes it mishandled.

The owner's direction replaces it with a positive rule: **the nearest binding decides, and an alias is followed to what it names.** This PR adds the following of `const` aliases. It is deliberately the positive half only: it adds edges and replaces a wrong edge where the alias is now proven. It does NOT yet stop the by-name search for aliases it does not follow; that is the next slice (already being implemented on a branch stacked on this one) and is out of scope here.

`Exact` in prism is a static-binding grade (see `CLAUDE.md`, call-resolution section): the name resolves, by the language's declaration, scope, import and export syntax as prism models it, to exactly one callable. Runtime mutation is out of model.

## What to read

Worktree: `/Users/wesleyjinks/code/prism-js-alias`, branch `feat/js-alias-follow`, head `3dd5b7db`; `git diff origin/main...HEAD` (the branch contains a merge of main; use the three-dot form). Pull request: `#372 (`gh pr view 372`, `gh pr diff 372`)`.
- Binder: `src/ast/js_binding_alias.rs` (new: `js_ts_follow_alias`, the link shapes, the bound), read together with `src/ast/js_binding.rs` (`js_ts_classify`: what makes a binding `Alias`, `MayCall`, `Callable`, `Import`), `js_binding_site.rs`, `js_binding_walk.rs`, `js_binding_namespace.rs`, `js_binding_writes.rs`.
- Site mapping and persisted state: `src/call_graph.rs` (`JsLocalBinding::{AliasCallable, AliasImport, AliasNamespaceMember}`, `js_local_binding_at`, and the stand-down in `compute_indirect_call_sites`), cache versions in `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs`.
- Resolver: `src/resolution_js_alias.rs` (new), and in `src/resolution.rs` the `None`-qualifier arm of `resolve_call_site` and `js_ts_import_member_candidates`.
- Tests: `src/ast/js_binding_alias_tests.rs`, `tests/integration/js_alias_follow_test.rs`, the pinned matrix `tests/fixtures/js_alias_matrix/` (read its README; `git diff origin/main...HEAD -- tests/fixtures/js_alias_matrix/expected.jsonl` is the row-by-row behavior change), mutation lane `mutants/js-alias-follow.json`.

## The contract as implemented

**Which declarations are followed.** Only when the binder's existing classification of the name at the site is `Alias` (so: the nearest declaring scope, one declaration, clean scope, not written anywhere in scope) AND the declaration is a `const` declarator whose value, after unwrapping parentheses and TypeScript assertions (not assignment chains), is: (a) an identifier; (b) a plain member `Q.m` of an identifier `Q` (no optional chain, no computed access); or (c) an identifier `Q` destructured by a flat object pattern in which the name is bound as shorthand `{ m }` or rename `{ m: a }` (no default anywhere in the pattern, no nesting, no rest, no computed or string key).

**What a link resolves to.** For (b) and (c), `Q` must be proven, by the existing namespace-import proof, to be one unwritten program-scope namespace import: the result is "member `m` of that module". For (a), the right-hand identifier is looked up by the binder at its own position: a proven callable → that callable; a single program-scope import declaration → "that import local"; another followed alias → continue; anything else → not followed. At most 8 links; a visited set; all links in one file.

**How the site resolves.** In-file callable: as a directly bound callable does today (by registered name and span; a wrapped React value binds from JSX only). Import local: as a direct call to that import through the JS/TS import-member route, without the caller-locals shadow guard on the site's spelling; no candidate → dropped. Namespace member: through the named-export proof for (module, member), i.e. as `import { m } from P` would resolve; relative modules only; no proven export, zero or several identities, a non-relative or unresolvable module → dropped. A followed alias never falls to the same-file or repo-wide name rungs.

**Two decisions made during implementation that you should judge:**
1. The namespace-member terminal uses the per-export proof that named imports use, NOT the route a qualified call `N.m()` takes today. The qualified route falls back to a file-stem match graded Exact when its (whole-module) namespace proof is unavailable; that fallback was shown to pick unrelated files (issue #371, item 1) and this PR does not extend it to alias sites. Consequence: `const h = N.f; h()` and `N.f()` can differ when the module's namespace proof is incomplete or the stem fallback fires.
2. Main has an older, language-generic textual alias path ("Level 1: local variable function pointer" in `compute_indirect_call_sites`, using `resolve_fptr_assignment`): it reads `h = f` from source text and emits an extra `IndirectResolution` record. Where the binder follows the alias, that path now stands down for the site, so one call yields one edge instead of two. Where the binder does not follow, the textual path is unchanged, including its known scope-blind false Exact (pinned in the matrix; the next slice's job).

**Not changed:** module export summaries (an exported alias `export const x = f;` stays unproven); every site whose binding is not a followed alias; non-JS output.

## Measured effect (controller; release binary of the head against the release binary of main `38a59994`, every `call-stats --dump-sites` record compared)

X = Excalidraw, T = the TypeScript compiler's `src`, F = a private React application (aggregates only).

| | X | T | F |
|---|---:|---:|---:|
| Call-site records compared | 19,219 | 61,712 | 13,299 |
| Sites that gained an Exact edge | 1 | 0 | 3 |
| … confirmed | 1 (TypeScript 5.9.3 checker agrees on the declaration) | n/a | 3 (module file and export name derived independently from syntax and the app's tsconfig path aliases) |
| Record added or removed; existing target changed or removed; any site resolved → dropped | 0 | 0 | 0 |

The corpus yield is small, as an earlier census predicted: plain `const` aliases of imports are rare in real code. The value claimed for this slice is the rule (the binding decides), which the next slices build on, and the matrix rows. On the matrix (1,169 fixtures): 239 rows MISSING → CORRECT, 39 WRONG → CORRECT, 2 WRONG → MISSING, 27 field-only; no row regresses by the guard's definition; 94 `WRONG` rows remain (56 through the repo-wide name search, 30 through the same-file by-name rung, 8 through the textual path), all on bindings this slice does not follow.

## Questions

1. **Wrong edges.** Name any input where a followed alias yields an Exact edge to a function the site does not statically call. Consider: shadowing of the alias name and of the right-hand name at every scope kind (parameters, catch, blocks, class bodies, `for` heads, switch cases, namespaces, hoisted `function`/`var`, labels); writes the binder might not see (compound assignment, `++`, destructuring assignment, writes in nested closures, `delete`, `with`, `eval` is out of model); TDZ and declaration order; default and namespace imports used as callables; type-only imports; re-exports and barrels; JSX vs call for wrapped React values; parse-error recovery; duplicate declarations; generated or minified code.
2. **Lost edges.** Name any input where a site that resolves correctly on main no longer does (the bless guard found no pinned row leaving `CORRECT`; the corpora show none). In particular: a followed alias whose terminal is unresolvable is now dropped instead of reaching the name rungs; is there a case where the name rung's edge was the provably right one?
3. **The two decisions above.** Are they right? For decision 1, would you rather mirror the qualified route? State the input and the result under each choice. For decision 2, is standing down safe for every consumer of `IndirectResolution` records (CPG call edges, data-flow argument edges, navigation)?
4. **The bound, the visited set and re-entrancy.** Can the follow recurse without bound or loop? Is 8 the right kind of limit (a row pins 8 succeeds, 9 does not)?
5. **Persisted state.** The three new `JsLocalBinding` variants, cache versions 116 and 64, round-trip and version-miss tests: complete?
6. **Tests and mutants.** Does every conjunct of the contract have a test that fails without it? The lane has 34 mutants, all killed; which behavior-changing mutant would survive? Three were judged equivalent by the implementer (the visited set, the `lexical_declaration` parent check, matching `(Import, Some)` vs `(_, Some)`): do you agree?
7. **The matrix diff.** 307 rows change: 280 verdict moves (278 to `CORRECT`; 2 `WRONG` → `MISSING` for an alias of an exported const alias) and 27 field-only changes (kind `import_qualified` → `import_member`; a drop reason; the `IndirectResolution` origin disappearing where the alias is followed). No ledger entry. Sample the diff: is any changed row wrong, and is any row that should have changed unchanged?
8. **Docs.** Is the `CLAUDE.md` sentence accurate?

## How to report

Tag every finding **WRONG** (the code provably does the wrong thing: name the input or state and the incorrect result, on this branch and on main) or **SMELL** (a risk or gap with no demonstrated incorrect behavior). A finding without a concrete failure scenario is a SMELL. Also tag each **MATERIAL** or **IMMATERIAL** to the goal above. List WRONG items first. A wrong edge that exists identically on main and is not made more reachable by this PR is out of scope: record it in one line as a follow-up, not as a finding against this PR.

For each finding give: the evidence (file and line, or the fixture you ran and its output on this branch AND on main); a recommended fix and at least one alternative with its cost; and when the finding would not apply.

End with a verdict: **APPROVE**, **FIX** (list what must change before merge) or **REJECT**, then "checked and found sound" and "not checked" lists.

## Rules

- You may read anything in the worktree and run its tests (`cargo test --lib js_binding_alias`, `cargo test --test integration js_alias`). To compare behavior on small synthetic fixtures you write, use the two prebuilt release binaries (do not rebuild): this branch `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/L2-prism`, main `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/main-38a59994-prism`; run `<binary> nav --cache-dir <fresh dir in your scratch> call-stats --repo <fixture dir> --dump-sites` and read the JSON lines whose `record_kind` is `call_site`.
- Do NOT run the matrix test with `PRISM_BLESS_JS_ALIAS_MATRIX` set in the worktree.
- Write scratch files only under your scratch directory (given below). Do not modify, add or delete any file in the worktree; make no git writes of any kind and post nothing to GitHub.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-`, `f-alias-` or `F-private`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
